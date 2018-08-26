/*
 * This file is a part of Exopticon, a free video surveillance tool. Visit
 * https://exopticon.org for more information.
 *
 * Copyright (C) 2018 David Matthew Mattli
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
#include <turbojpeg.h>
#include <unistd.h>

#include <arpa/inet.h>

#include "exvid.h"
#include "mpack_frame.h"

static const int MAX_FILE_SIZE = 10 * 1024 * 1024;

int64_t timespec_to_ms(const struct timespec time);
char *timespec_to_8601(struct timespec *ts);

struct CameraState {
        time_t timenow, timestart;
        int got_key_frame;

        struct in_context in;
        struct out_context out;

        AVFormatContext *ofcx;
        AVOutputFormat *ofmt;
        AVCodecContext *occx;
        AVCodec *ocodec;
        AVStream *ost;
        int frames_per_second;
        int o_index;
        // Begin, End time for output file
        struct timespec begin_time;
        struct timespec end_time;
        char *output_directory_name;

        // Begin, End time for frame
        struct timespec frame_begin_time;
        struct timespec frame_end_time;
        int64_t first_pts, last_pts;

        AVPacket pkt;
        AVFrame *frame;

        int64_t file_size;
        int switch_file;
};

struct FrameTime {
        int64_t decode_time;
        int64_t file_write_time;
        int64_t stream_write_time;
        int64_t read_time;
        int64_t whole_loop;
};

struct CapturePerformance {
        struct FrameTime frame_times[50];
        int count;

        struct FrameTime min;
        struct FrameTime max;
        struct FrameTime avg;
};

static pthread_mutex_t log_mutex = PTHREAD_MUTEX_INITIALIZER;

static void my_av_log_callback(__attribute__((unused)) void *avcl, int level, const char *fmt,
                               va_list vl)
{
        char output_message[2048];

        if (av_log_get_level() < level) {
                return;
        }
        pthread_mutex_lock(&log_mutex);

        vsnprintf(output_message, sizeof(output_message), fmt, vl);
        send_log_message(level, output_message);

        pthread_mutex_unlock(&log_mutex);
        return;
}

void bs_log(const char *const fmt, ...)
{
        va_list ap;
        va_start(ap, fmt);
        my_av_log_callback(NULL, AV_LOG_INFO, fmt, ap);
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

int encode_jpeg(AVFrame *frame, AVPacket *pkt)
{
        AVCodec *codec = NULL;
        AVCodecContext *ccx = NULL;
        enum AVPixelFormat img_fmt = AV_PIX_FMT_YUVJ420P;
        int ret = 0;

        // Find the mjpeg encoder
        codec = avcodec_find_encoder(AV_CODEC_ID_MJPEG);
        if (!codec) {
                bs_log("Could not find codec");
                ret = 1;
                goto cleanup;
        }

        ccx = avcodec_alloc_context3(codec);
        if (ccx == NULL) {
                bs_log("Could not allocate codec context!");
                ret = 1;
                goto cleanup;
        }

        ccx->width = frame->width;
        ccx->height = frame->height;
        ccx->pix_fmt = img_fmt;

        // Set quality
        ccx->qmin = 1;
        ccx->qmax = 15;
        ccx->mb_lmin = ccx->qmin * FF_QP2LAMBDA;
        ccx->mb_lmax = ccx->qmax * FF_QP2LAMBDA;

        ccx->time_base.num = 5;
        ccx->time_base.den = 1;

        pkt->data = NULL;
        pkt->size = 0;
        if (avcodec_open2(ccx, codec, NULL) < 0) {
                bs_log("Failed to open codec");
                ret = 1;
                goto cleanup;
        }

        frame->pts = 1;
        frame->quality = ccx->global_quality;
        frame->format = img_fmt;
        frame->width = ccx->width;
        frame->height = ccx->height;

        int send_ret = avcodec_send_frame(ccx, frame);
        if (send_ret != 0) {
                bs_log("Error sending frame.");
                ret = 1;
                goto cleanup;
        }
        int receive_ret = avcodec_receive_packet(ccx, pkt);
        if (receive_ret != 0) {
                bs_log("Error receiving pkt.");
                ret = 2;
                goto cleanup;
        }
cleanup:
        avcodec_close(ccx);
        av_free(ccx);
        return ret;
}

int write_jpeg(AVPacket *pkt, const char *filename)
{
        FILE *JPEGFile;
        char JPEGFName[256];

        snprintf(JPEGFName, sizeof(JPEGFName), "%s.tmp", filename);
        JPEGFile = fopen(JPEGFName, "wb");
        if (JPEGFile == NULL) {
                bs_log("Unable to open %s to write jpg. Error: %s", JPEGFName,
                       strerror(errno));
                return 1;
        }
        fwrite(pkt->buf->data, 1, pkt->size, JPEGFile);
        fclose(JPEGFile);

        rename(JPEGFName, filename);

        return 0;
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
        bs_log("Generated filename: %s", name);
        if (ret < 0 || ret > size) {
                // An error occured...
                bs_log("Error creating output filename, snprintf "
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

int close_output_file(struct CameraState *cam)
{
        int ret = 0;
        char *end_time;
        char filename[1024];
        FILE *output_file = NULL;

        ret = clock_gettime(CLOCK_REALTIME, &(cam->end_time));
        if (ret == -1) {
                // error!
        }

        end_time = timespec_to_8601(&(cam->end_time));

        strncpy(filename, cam->ofcx->filename, sizeof(filename));
        filename[sizeof(filename) - 1] = '\0';

        av_write_trailer(cam->ofcx);
        avio_close(cam->ofcx->pb);
        avformat_free_context(cam->ofcx);
        cam->ofcx = NULL;

        /* We need to set the ENDTIME tag in the output file but
         * ffmpeg only lets us set tags before calling
         * avformat_write_header and that has to be done before
         * writing anything. So instead we set a dummy tag ENDTIME tag
         * and overwrite it manually with fwrite. This only works
         * because the replacement tag value is exactly the same size
         * as the dummy value.
         */

        output_file = fopen(filename, "r+");
        if (output_file == NULL) {
                ret = -1;
                goto cleanup;
        }

        int64_t pos = find_string_in_file(output_file, "ENDTIMED");
        if (pos > 0) {
                fseek(output_file, pos + 3, SEEK_SET);
                fwrite(end_time, strlen(end_time), 1, output_file);
        }

        // Report file as finished
        report_finished_file(filename, cam->end_time);

cleanup:
        if (output_file != NULL) {
                fclose(output_file);
        }
        free(end_time);

        return ret;
}

int64_t timespec_to_ms(const struct timespec time)
{
        const int64_t million = 1E6;

        const int64_t time_ms = time.tv_sec * 1000 + (time.tv_nsec / million);

        return time_ms;
}

AVFrame* scale_frame(AVFrame *input, int width, int height)
{
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
                                                              AV_PIX_FMT_YUV420P,//input->format,
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

int send_scaled_frame(AVFrame *frame, const int offset, const int width, const int height)
{
        struct FrameMessage message;
        message.offset = offset;

        AVPacket jpeg_pkt;
        av_init_packet(&jpeg_pkt);

        AVFrame *scaledFrame = NULL;
        scaledFrame = scale_frame(frame, width, height);

        // jpeg encode frame
        encode_jpeg(scaledFrame, &jpeg_pkt);
        message.jpeg = jpeg_pkt.buf->data;
        message.jpeg_size = jpeg_pkt.buf->size;

        send_scaled_frame_message(&message, scaledFrame->height);

        av_freep(&(scaledFrame->data));
        av_frame_free(&scaledFrame);
        av_packet_unref(&jpeg_pkt);
        return 0;
}

int send_full_frame(AVFrame *frame, const int offset)
{
        struct FrameMessage message;
        message.offset = offset;

        AVPacket jpegPkt;
        av_init_packet(&jpegPkt);
        encode_jpeg(frame, &jpegPkt);

        message.jpeg = jpegPkt.buf->data;
        message.jpeg_size = jpegPkt.buf->size;
        message.offset = offset;

        send_frame_message(&message);

        av_packet_unref(&jpegPkt);

        return 0;
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
                        bs_log("Error closing output stream!");
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
                char *fn = generate_output_name(output_directory, get_time());
                struct timespec begin_time;
                clock_gettime(CLOCK_REALTIME, &begin_time);
                bs_log("Opening file: %s\n", fn);
                int ret = ex_open_output_stream(in, out, fn);
                if (ret != 0) {
                        bs_log("Error opening output stream!");
                        return 2;
                }
                pkt->stream_index = in->stream_index;
                report_new_file(fn, begin_time);
        }

        return 0;
}

int push_frame(struct in_context *in, struct out_context *out, AVPacket *pkt)
{
        // Make sure packet is video
        if (pkt->stream_index != in->stream_index) {
                return 0;
        }

        AVFrame *frame = av_frame_alloc();

        int send_ret = avcodec_send_packet(in->ccx, pkt);
        if (send_ret != 0) {
                char errbuf[200];
                av_strerror(send_ret, errbuf, sizeof errbuf);
                bs_log("Error decoding frame! %s\n", errbuf);
                goto cleanup;
        }

        int receive_ret = avcodec_receive_frame(in->ccx, frame);
        if (receive_ret != 0) {
                char errbuf[100];
                av_strerror(receive_ret, errbuf, 100);
                av_log(NULL, AV_LOG_FATAL, "Error receiving frame: %s", errbuf);
                goto cleanup;
        }
        AVRational nsec = av_make_q(1, 1E9); // one billion
        struct timespec pts_ts;
        pts_ts.tv_sec = ((frame->pts - out->first_pts) * in->st->time_base.num) / in->st->time_base.den;
        int64_t frac_sec =
          (frame->pts - out->first_pts) - ((pts_ts.tv_sec * in->st->time_base.den) / in->st->time_base.num);
        pts_ts.tv_nsec =
          av_rescale_q(frac_sec, in->st->time_base, nsec);

        // calculate offset in milliseconds
        const int offset_ms = (pts_ts.tv_sec * 1000) + (pts_ts.tv_nsec / 1000000);

        send_full_frame(frame, offset_ms);
        send_scaled_frame(frame, offset_ms, 640, 360);

cleanup:
        av_frame_free(&frame);
        return 0;
}

int checkforquit()
{
        int ret = 0;
        int quit = 0;
        char buf[1024];
        fd_set rfds;
        struct timeval timeout;
        timeout.tv_sec = 0;
        timeout.tv_usec = 0;

        FD_ZERO(&rfds);
        FD_SET(0, &rfds);

        do {
                ret = select(1, &rfds, NULL, NULL, &timeout);
                size_t read_size = 0;
                if (ret > 0 && FD_ISSET(0, &rfds)) {

                        // select return value is greater than 0 and stdin is in
                        // the set, read!
                        read_size = fread(buf, 1, 1, stdin);
                }

                if (feof(stdin) != 0) {
                        quit = 1;
                }



                // TODO change second loop condition to assert
                for (size_t i = 0; i < read_size && i < sizeof(buf); ++i) {
                        if (buf[i] == 'q') {
                                quit = 1;
                                break;
                        }
                }
        } while (ret > 0 && quit == 0);


        return quit;
}

int main(int argc, char *argv[])
{
        int return_value = EXIT_SUCCESS;
        struct CameraState cam;
        cam.ofcx = NULL;
        cam.frame = NULL;

        const char *program_name = argv[0];
        const char *input_uri;

        if (argc < 5) {
                fprintf(stderr, "Usage: %s url frames_per_second "
                                "output_directory jpg_name\n",
                        program_name);
                return 1;
        }
        input_uri = argv[1];
        cam.frames_per_second = atoi(argv[2]);
        cam.output_directory_name = argv[3];

        // Seed rand
        srand(time(NULL));

        // Initialize library
        ex_init(my_av_log_callback);
        ex_init_input(&cam.in);
        ex_init_output(&cam.out);

        cam.ofcx = NULL;
        cam.got_key_frame = 0;
        cam.frame = av_frame_alloc();
        cam.file_size = 0;

        int open_ret = ex_open_input_stream(input_uri, &cam.in);
        if (open_ret> 0) {
                bs_log("Error opening stream, %s, error %d!", input_uri, open_ret);
                goto cleanup;
        }

        av_init_packet(&cam.pkt);
        cam.switch_file = 1;
        int ret = 0;
        while ((ret = ex_read_frame(&cam.in, &cam.pkt)) >= 0) {
                if (cam.pkt.stream_index == cam.in.stream_index) {
                        int handle_ret = handle_output_file(&cam.in, &cam.out, &cam.pkt, cam.output_directory_name);
                        if (handle_ret != 0) {
                                bs_log("Handle error!");
                                goto cleanup;
                        }
                        if (cam.out.fcx == NULL) {
                                continue;
                        }
                        push_frame(&cam.in, &cam.out, &cam.pkt);
                        int write_ret = ex_write_output_packet(&cam.out, cam.in.st->time_base, &cam.pkt);
                        if (write_ret != 0) {
                                bs_log("Write Error!");
                                goto cleanup;
                        }
                }
                av_packet_unref(&cam.pkt);
                av_init_packet(&cam.pkt);
        }

        char buf[1024];
        av_strerror(ret, buf, sizeof(buf));
        bs_log("av_read_frame returned %d, %s exiting...", ret, buf);

cleanup:
        bs_log("Cleaning up!");
        ex_free_input(&cam.in);
        if (cam.ofcx != NULL) {
                close_output_file(&cam);
        }

        av_frame_free(&cam.frame);

        avformat_network_deinit();

        return return_value;
}
