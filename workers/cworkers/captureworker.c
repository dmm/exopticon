/* * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

#if __STDC_VERSION__ >= 199901L
#define _XOPEN_SOURCE 600
#else
#define _XOPEN_SOURCE 500
#endif /* __STDC_VERSION__ */

#include <assert.h>
#include <fcntl.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixfmt.h>
#include <libswscale/swscale.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include <arpa/inet.h>

#include "exvid.h"
#include "mpack_frame.h"

#define METRIC_SAMPLES 25 * 5

enum metrics {
        LOOP_TIME,
        DECODE_TIME,
        JPEG_SCALED,
        JPEG_FULL,
        SERIALIZE_SCALED,
        SERIALIZE_FULL,
        METRIC_COUNT
};

static char *metric_labels[METRIC_COUNT] = {
        "loop_time",
        "decode_time",
        "scaled_jpeg_encode_time",
        "jpeg_encode_time",
        "scaled_jpeg_serialize_time",
        "scaled_jpeg_serialize_time",
};

static const int MAX_FILE_SIZE = 10 * 1024 * 1024;

int64_t timespec_to_ms(const struct timespec time);
char *timespec_to_8601(struct timespec *ts);
void ex_log(int level, const char *const fmt, ...);

struct CameraState {
        time_t timenow, timestart;
        int got_key_frame;

        struct in_context in;
        struct out_context out;

        // Begin, End time for output file
        char *output_directory_name;

        AVPacket pkt;

        // capture metrics
        int metric_index;
        double metrics[METRIC_COUNT][METRIC_SAMPLES];
        struct timespec metric_start_time[METRIC_COUNT][METRIC_SAMPLES];
};

static void record_metric_start(struct CameraState *cam, enum metrics metric) {
        assert(metric < METRIC_COUNT);
        assert(cam->metric_index < METRIC_SAMPLES);

        clock_gettime(CLOCK_MONOTONIC, &(cam->metric_start_time[metric][cam->metric_index]));
}

static void record_metric_end(struct CameraState *cam, enum metrics metric) {
        assert(metric < METRIC_COUNT);
        assert(cam->metric_index < METRIC_SAMPLES);

        struct timespec end;
        clock_gettime(CLOCK_MONOTONIC, &end);

        int64_t interval_ms = timespec_to_ms_interval(cam->metric_start_time[metric][cam->metric_index],
                                                      end);

       cam->metrics[metric][cam->metric_index] = (double)interval_ms;
}

static int is_hwaccel_pix_fmt(enum AVPixelFormat pix_fmt)
{
        const AVPixFmtDescriptor *desc = av_pix_fmt_desc_get(pix_fmt);
        return desc->flags & AV_PIX_FMT_FLAG_HWACCEL;
}

static const AVRational microsecond =
{
        .num = 1,
        .den = 1E6,
};

static pthread_mutex_t log_mutex = PTHREAD_MUTEX_INITIALIZER;

static void my_av_log_callback(__attribute__((unused)) void *avcl, int level, const char *fmt,
                               va_list vl)
{
        char output_message[2048];

        pthread_mutex_lock(&log_mutex);

        vsnprintf(output_message, sizeof(output_message), fmt, vl);
        send_log_message(level, output_message);

        pthread_mutex_unlock(&log_mutex);
        return;
}

void ex_log(int level, const char *const fmt, ...)
{
        va_list ap;
        va_start(ap, fmt);
        my_av_log_callback(NULL, level, fmt, ap);
}

void report_new_file(char *filename, struct timespec begin_time)
{
        char *isotime = timespec_to_8601(&begin_time);
        send_new_file_message(filename, isotime);
        free(isotime);
}
void report_finished_file(char *filename, struct timespec end_time)
{
        char *isotime = timespec_to_8601(&end_time);
        send_end_file_message(filename, isotime);
        free(isotime);
}

time_t get_time()
{
        struct timeval tv;

        gettimeofday(&tv, NULL);

        return tv.tv_sec;
}

char *timespec_to_8601(struct timespec *ts)
{
        const int size = 60;
        char date[30];
        char frac_secs[30];
        char timezone[10];
        char *ret = calloc(size, 1);
        int result = 0;
        struct tm t;

        if (localtime_r(&(ts->tv_sec), &t) == NULL) {
                return NULL;
        }

        result = strftime(date, sizeof(date), "%FT%H:%M:%S", &t);
        if (result == 0) {
                goto error;
        }

        result = snprintf(frac_secs, sizeof(frac_secs), ".%03ld", ts->tv_nsec);
        if (result < 0) {
                goto error;
        }

        result = strftime(timezone, sizeof(timezone), "%z", &t);
        if (result == 0) {
                goto error;
        }

        result = snprintf(ret, size, "%s%s%s", date, frac_secs, timezone);
        if (result < 0) {
                goto error;
        }

        return ret;
error:
        free(ret);
        return NULL;
}

char *generate_output_name(const char *output_directory_name, time_t time)
{
        int size = 256;
        char *name = calloc(1, size);
        char *isotime = calloc(1, size);
        int nonce = rand();

        strftime(isotime, size, "%FT%H%M%S%z", localtime(&time));

        int ret = snprintf(name, size, "%s/%lld_%s_%d.mkv", output_directory_name,
                           (long long)time, isotime, nonce);
        ex_log(AV_LOG_INFO, "Generated filename: %s", name);
        if (ret < 0 || ret > size) {
                // An error occured...
                ex_log(AV_LOG_ERROR, "Error creating output filename, snprintf "
                       "return value: %d",
                       ret);
        }

        free(isotime);
        return name;
}

int64_t find_string_in_file(FILE *file, char *string)
{
        int64_t pos = 0;
        int64_t cur_string_idx = 0;
        const int length = strlen(string);
        int found = 0;

        char temp;

        fseek(file, 0L, SEEK_SET);

        while (fread(&temp, 1, 1, file) == 1) {
                if (temp == string[cur_string_idx]) {
                        cur_string_idx++;
                }
                if (cur_string_idx >= length) {
                        found = 1;
                        break;
                }
                pos++;
        }

        if (found == 0) {
                pos = -1;
        }

        return pos;
}


int64_t timespec_to_ms(const struct timespec time)
{
        const int64_t million = 1E6;

        const int64_t time_ms = time.tv_sec * 1000 + (time.tv_nsec / million);

        return time_ms;
}

AVFrame* scale_frame(AVFrame *input, int width, int height)
{
        // Correct color space if using deprecated pix fmt
        if (input->format == AV_PIX_FMT_YUVJ420P) {
                input->format = AV_PIX_FMT_YUV420P;
                input->color_range = AVCOL_RANGE_JPEG;
        }
        AVFrame* resizedFrame = av_frame_alloc();
        if (resizedFrame == NULL) {
                return NULL;
        }

        resizedFrame->format = AV_PIX_FMT_YUV420P; //input->format;
        resizedFrame->width = width;
        resizedFrame->height = height;
        int ret = av_image_alloc(resizedFrame->data,
                                 resizedFrame->linesize,
                                 resizedFrame->width,
                                 resizedFrame->height,
                                 resizedFrame->format,
                                 32);
        if (ret < 0) {
                av_frame_free(&resizedFrame);
                return NULL;
        }

        struct SwsContext *sws_context = sws_getCachedContext(NULL,
                                                              input->width,
                                                              input->height,
                                                              input->format,
                                                              resizedFrame->width,
                                                              resizedFrame->height,
                                                              resizedFrame->format,
                                                              SWS_BICUBIC,
                                                              NULL,
                                                              NULL,
                                                              NULL);
        sws_scale(sws_context,
                  (const unsigned char* const*)input->data,
                  input->linesize,
                  0,
                  input->height,
                  resizedFrame->data,
                  resizedFrame->linesize);

        sws_freeContext(sws_context);
        return resizedFrame;
}


int handle_output_file(struct in_context *in, struct out_context *out, AVPacket *pkt, const char *output_directory)
{
        assert(in != NULL);
        assert(out != NULL);
        assert(pkt != NULL);

        // ensure packet is from selected video stream
        if (pkt->stream_index != in->stream_index) {
                return 1;
        }

        // Check if output file if oversized and we are have a
        // keyframe. If so close the output file.
        if (out->size >= MAX_FILE_SIZE && (pkt->flags & AV_PKT_FLAG_KEY)) {
                // close file
                int ret = ex_close_output_stream(out);
                if (ret != 0) {
                        ex_log(AV_LOG_ERROR, "Error closing output stream!");
                        return 1;
                }
                struct timespec end_time;
                clock_gettime(CLOCK_REALTIME, &end_time);
                report_finished_file(out->output_path, end_time);
                ex_init_output(out);
        }

        // If output is closed and we have a keyframe open new file.
        if (out->fcx == NULL && (pkt->flags & AV_PKT_FLAG_KEY)) {
                // Open file
                ex_init_output(out);
                out->first_pts = pkt->pts;
                char *fn = generate_output_name(output_directory, get_time());
                struct timespec begin_time;
                clock_gettime(CLOCK_REALTIME, &begin_time);
                ex_log(AV_LOG_ERROR, "Opening file: %s\n", fn);
                int ret = ex_open_output_stream(in, out, fn);
                if (ret != 0) {
                        ex_log(AV_LOG_ERROR, "Error opening output stream!");
                        return 2;
                }
                pkt->stream_index = in->stream_index;
                report_new_file(fn, begin_time);
        }

        return 0;
}


int main(int argc, char *argv[])
{
        int return_value = EXIT_SUCCESS;
        struct CameraState cam;

        const char *program_name = argv[0];
        const char *input_uri;

        if (argc < 4) {
                fprintf(stderr, "Usage: %s url output_directory hwaccel_type\n",
                        program_name);
                return 1;
        }
        input_uri = argv[1];
        cam.output_directory_name = argv[2];

        // Seed rand
        srand(time(NULL));

        // Initialize library
        ex_init(my_av_log_callback);
        ex_init_input(&cam.in);
        ex_init_output(&cam.out);

        cam.in.hw_accel_type = av_hwdevice_find_type_by_name(argv[3]);

        cam.got_key_frame = 0;


        int open_ret = ex_open_input_stream(input_uri, &cam.in);
        if (open_ret> 0) {
                ex_log(AV_LOG_ERROR, "Error opening stream, %s, error %d!", input_uri, open_ret);
                goto cleanup;
        }

        av_init_packet(&cam.pkt);
        int ret = 0;
        clock_gettime(CLOCK_MONOTONIC, &(cam.in.last_frame_time));
        while ((ret = ex_read_frame(&cam.in, &cam.pkt)) >= 0) {
                if (cam.pkt.stream_index == cam.in.stream_index) {
                        record_metric_start(&cam, LOOP_TIME);
                        int handle_ret = handle_output_file(&cam.in, &cam.out, &cam.pkt, cam.output_directory_name);
                        if (handle_ret != 0) {
                                ex_log(AV_LOG_ERROR, "Handle error!");
                                goto cleanup;
                        }
                        if (cam.out.fcx == NULL) {
                                ex_log(AV_LOG_ERROR, "FCX null???");
                                continue;
                        }

//                        push_frame(&cam);

                        ex_send_packet(&cam.in, &cam.pkt);
                        int write_ret = ex_write_output_packet(&cam.out, cam.in.st->time_base, &cam.pkt);
                        if (write_ret != 0) {
                                ex_log(AV_LOG_ERROR, "Write Error!");
                                goto cleanup;
                        }
                        clock_gettime(CLOCK_MONOTONIC, &(cam.in.last_frame_time));
                        record_metric_end(&cam, LOOP_TIME);
                } else {
                        // handle non-video ?
                }
                av_packet_unref(&cam.pkt);
                av_init_packet(&cam.pkt);
        }

        char buf[1024];
        av_strerror(ret, buf, sizeof(buf));
        ex_log(AV_LOG_ERROR, "av_read_frame returned %d, %s exiting...", ret, buf);

cleanup:
        ex_log(AV_LOG_INFO, "Cleaning up!");
        ex_free_input(&cam.in);

        avformat_network_deinit();

        return return_value;
}
