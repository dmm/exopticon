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

static void bs_print(const char *fmt, ...)
{
        va_list ap;
        char buf[2048];
        return;
        buf[0] = 0;
        pthread_mutex_lock(&log_mutex);

        va_start(ap, fmt);
        vsnprintf(buf, sizeof(buf), fmt, ap);
        buf[(sizeof buf) - 1] = '\0';
        const int32_t msg_length = strlen(buf);
        const int32_t be_msg_length = htonl(msg_length);
        fwrite(&be_msg_length, sizeof be_msg_length, 1, stderr);
        fwrite(buf, 1, msg_length, stderr);
        fflush(stderr);

        pthread_mutex_unlock(&log_mutex);
}

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
        my_av_log_callback(NULL, 1, fmt, ap);
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

        int got_frame = 0;
        int encode_ret = avcodec_encode_video2(ccx, pkt, frame, &got_frame);
        if (encode_ret != 0) {
                bs_log("Error encoding jpeg");
                ret = 1;
        }
        if (got_frame == 1) {
                ret = 0;
        } else {
                bs_log("got no frame :(");
                ret = 1;
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

int initialize_output_stream(struct CameraState *cam, const char *out_filename)
{
        // open output file
        // TODO !!! Verify this actually works!!!!!!
        cam->ofmt = av_guess_format(NULL, out_filename, NULL);
        cam->ofcx = avformat_alloc_context();
        cam->ofcx->oformat = cam->ofmt;
        avio_open2(&cam->ofcx->pb, out_filename, AVIO_FLAG_WRITE, NULL, NULL);

        // Create output stream
        cam->ost = avformat_new_stream(cam->ofcx, NULL);
        avcodec_copy_context(cam->ost->codec, cam->in.ccx);

        cam->ost->sample_aspect_ratio.num = cam->in.ccx->sample_aspect_ratio.num;
        cam->ost->sample_aspect_ratio.den = cam->in.ccx->sample_aspect_ratio.den;

        // Assume r_frame_rate is accurate
        bs_log("cam->ist ticks_per_fram: %d", cam->in.st->codec->ticks_per_frame);

        AVRational output_timebase;
        AVRational output_fps;
        if (cam->frames_per_second > 0) {
                output_fps.num = cam->frames_per_second;
                output_fps.den = 1;

                output_timebase.num = 1;
                output_timebase.den = 1000;
        } else if (cam->frames_per_second == 0) {  // cam->frames_per_second == 0

                output_fps = cam->in.st->r_frame_rate;
                output_timebase.num = cam->in.st->time_base.num;
                output_timebase.den = cam->in.st->time_base.den;
        } else { // cam->frames_per_second < 0
                bs_log("Invalid frames_per_second: %d\n", cam->frames_per_second);
                exit(1);
        }
        cam->ost->avg_frame_rate = output_fps;
        // cam->ost->time_base = av_inv_q(cam->ost->r_frame_rate);
        cam->ost->time_base = output_timebase;

        // The cam->ost time_base is 1/fps, so codec->ticks_per_frame should be
        // one. Otherwise the video plays at twice the normal speed.
        //      cam->ost->codec->ticks_per_frame = 1;
        bs_log("cam->ost ticks_per_frame: %d ",
               cam->ost->codec->ticks_per_frame);

        // Set global headers
        cam->ost->codec->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;

        // Set file begin time
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        char *timestring = timespec_to_8601(&ts);

        av_dict_set(&(cam->ofcx->metadata), "ENDTIME", timestring, 0);
        av_dict_set(&(cam->ofcx->metadata), "BEGINTIME", timestring, 0);

        free(timestring);

        int ret = avformat_write_header(cam->ofcx, NULL);
        if (ret != 0) {
                return 1;
        }

        cam->last_pts = 0;
        cam->first_pts = (int64_t)LLONG_MAX;

        snprintf(cam->ofcx->filename, sizeof(cam->ofcx->filename), "%s",
                 out_filename);
        return 0;
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

int send_scaled_frame(AVFrame *frame, const int pts, const int width, const int height)
{
        struct FrameMessage message;
        message.pts = pts;

        AVPacket jpeg_pkt;
        av_init_packet(&jpeg_pkt);

        AVFrame *scaledFrame = NULL;
        scaledFrame = scale_frame(frame, width, height);

        // jpeg encode frame
        encode_jpeg(scaledFrame, &jpeg_pkt);
        message.jpeg = jpeg_pkt.buf->data;
        message.jpeg_size = jpeg_pkt.buf->size;

        send_scaled_frame_message(&message, scaledFrame->height);

cleanup:
        av_freep(&(scaledFrame->data));
        av_frame_free(&scaledFrame);
        av_packet_unref(&jpeg_pkt);
}

int send_full_frame(AVFrame *frame, const int pts)
{
        struct FrameMessage message;
        message.pts = pts;

        AVPacket jpegPkt;
        av_init_packet(&jpegPkt);
        encode_jpeg(frame, &jpegPkt);

        message.jpeg = jpegPkt.buf->data;
        message.jpeg_size = jpegPkt.buf->size;
        message.pts = pts;

        send_frame_message(&message);
//        send_scaled_frame(frame, pts, 640, 360);

        av_packet_unref(&jpegPkt);

        return 0;
}

int handle_packet2(struct in_context *in, struct out_context *out, AVPacket *pkt)
{
        if (pkt->stream_index != in->stream_index) {
                // ensure packet is from selected video stream
                return 0;
        }

        if (out->size > MAX_FILE_SIZE) {
//                char *fn = generate_output_name 
        }
        return 0;
}

int handle_output_file(struct in_context *in, struct out_context *out, AVPacket *pkt, const char *output_directory)
{
        assert(in != NULL);
        assert(out != NULL);
        assert(pkt != NULL);

        if (pkt->stream_index != in->stream_index) {
                // ensure packet is from selected video stream
                return 1;
        }


        if (out->st == NULL && !(pkt->flags & AV_PKT_FLAG_KEY)) {
                // Wait for keyframe
                return 1;
        }

        if (out->st == NULL && (pkt->flags & AV_PKT_FLAG_KEY)) {
                // Open file for first time
                ex_init_output(out);
                char *fn = generate_output_name(output_directory, get_time());
                ex_open_output_stream(out,
                                      in->codecpar,
                                      in->st->sample_aspect_ratio,
                                      fn);

        }


}

int handle_packet(struct CameraState *cam)
{
        // Make sure packet is video
        if (cam->pkt.stream_index != cam->in.stream_index) {
                return 0;
        }
        // Make sure we start on a key frame
        if (cam->got_key_frame == 0 && !(cam->pkt.flags & AV_PKT_FLAG_KEY)) {
                cam->timestart = cam->timenow = get_time();
                return 0;
        }

        if (cam->switch_file == 1 && (cam->pkt.flags & AV_PKT_FLAG_KEY)) {
                char *fn = NULL;
                fn = generate_output_name(cam->output_directory_name, get_time());
                if (cam->ofcx != NULL) {
                        clock_gettime(CLOCK_REALTIME, &(cam->end_time));
                        close_output_file(cam);
                }
                initialize_output_stream(cam, fn);

//                av_dump_format(cam->ofcx, 0, cam->ofcx->filename, 1);
                cam->switch_file = 0;
                cam->file_size = 0;
                clock_gettime(CLOCK_REALTIME, &(cam->begin_time));
                report_new_file(fn, cam->begin_time);
                free(fn);
        }

        if (cam->file_size > MAX_FILE_SIZE) {
                bs_log("File size is bigger than max! %lld",
                       (long long)cam->file_size);
                cam->switch_file = 1;
        }
        cam->got_key_frame = 1;

        if (cam->first_pts < cam->pkt.pts) {
                cam->first_pts = cam->pkt.pts;
        }
        cam->pkt.stream_index = cam->ost->id;
        cam->pkt.pts -= cam->first_pts;
        cam->pkt.pts = av_rescale_q(cam->pkt.pts,
                                    cam->in.st->time_base,
                                    cam->ost->time_base);
        cam->pkt.dts = cam->pkt.pts;

        AVPacket decode_packet;
        struct timespec write_begin;
        struct timespec write_end;
        cam->file_size += cam->pkt.size;
        av_init_packet(&decode_packet);
        av_packet_ref(&decode_packet, &(cam->pkt));
        clock_gettime(CLOCK_MONOTONIC, &write_begin);
        av_interleaved_write_frame(cam->ofcx, &cam->pkt);

        clock_gettime(CLOCK_MONOTONIC, &write_end);

        int got_frame = 0;
        int len = avcodec_decode_video2(cam->in.ccx, cam->frame, &got_frame,
                                        &decode_packet);

        if (len <= 0 || got_frame == 0) {
                bs_log("Error decoding frame! len %d, got_frame: %d", len,
                       got_frame);
                goto cleanup;
        }

        send_full_frame(cam->frame, cam->frame->pts);
        send_scaled_frame(cam->frame, cam->frame->pts, 640, 360);


cleanup:
        av_packet_unref(&decode_packet);

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

        cam.ofcx = NULL;
        cam.got_key_frame = 0;
        cam.frame = av_frame_alloc();
        cam.file_size = 0;


        int open_ret = ex_open_input_stream(input_uri, &cam.in);
        if (open_ret> 0) {
                bs_log("Error opening stream, %s, error %d!", input_uri, open_ret);
                goto cleanup;
        }

        // Initialize output stream size
        cam.out.size = (int64_t)LLONG_MAX;

        av_init_packet(&cam.pkt);
        cam.switch_file = 1;
        int ret = 0;
        while ((ret = ex_read_frame(&cam.in, &cam.pkt)) >= 0) {
                handle_packet(&cam);
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
