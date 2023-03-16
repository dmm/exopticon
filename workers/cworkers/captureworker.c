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
#include <turbojpeg.h>
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
void bs_log(const char *const fmt, ...);

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

static void increment_metrics(struct CameraState *cam) {
        assert(cam->metric_index < METRIC_SAMPLES);

        cam->metric_index++;

        if (cam->metric_index >= METRIC_SAMPLES) {
                // report metrics
                bs_log("LOOP TIME: %f, DECODE_TIME: %f, JPEG FULL: %f, JPEG SCALED: %f\n", cam->metrics[LOOP_TIME][0],
                       cam->metrics[DECODE_TIME][0],
                       cam->metrics[JPEG_FULL][0],
                       cam->metrics[JPEG_SCALED][0]);
                cam->metric_index = 0;
        }
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

void free_buffer(void *opaque, uint8_t *data)
{
        // Silence unused parameter warning for callback
        (void)(opaque);

        tjFree(data);
}

int encode_jpeg_turbo(AVFrame *frame, AVPacket *pkt)
{
        const int JPEG_QUALITY = 80;

        unsigned char* output_buf = NULL;
        long unsigned int jpeg_size = 0;
        tjhandle jpeg_compressor = tjInitCompress();

        const unsigned char* planes[3] = {frame->data[0], frame->data[1], frame->data[2]};
        int strides[3] = {frame->linesize[0], frame->linesize[1], frame->linesize[2]};
        tjCompressFromYUVPlanes(jpeg_compressor,
                                planes,
                                frame->width,
                                strides,
                                frame->height,
                                TJSAMP_420,
                                &output_buf,
                                &jpeg_size,
                                JPEG_QUALITY,
                                TJFLAG_FASTDCT);

        pkt->buf = av_buffer_create(output_buf, jpeg_size, &free_buffer, NULL, AV_BUFFER_FLAG_READONLY);

        tjDestroy(jpeg_compressor);
        return 0;
}

int encode_jpeg_ffmpeg(AVFrame *frame, AVPacket *pkt)
{
        AVCodec *codec = NULL;
        AVCodecContext *ccx = NULL;
        enum AVPixelFormat img_fmt = AV_PIX_FMT_YUV420P;
        int ret = 0;
        bs_log("Frame format: %d", frame->format);
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

int encode_jpeg(AVFrame *frame, AVPacket *pkt)
{
// frame->format is now expected to be AV_PIX_FMT_YUVJ420P
        if (frame->format != AV_PIX_FMT_YUV420P) {
                bs_log("Invalid frame format %s provided!",  av_get_pix_fmt_name(frame->format));
                return 1;
        }
        return encode_jpeg_turbo(frame, pkt);
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

int send_scaled_jpeg(AVPacket *jpeg_pkt, const int64_t offset, __attribute__ ((unused)) const int width, const int height, const int unscaled_width, const int unscaled_height)
{
        struct FrameMessage message;
        message.unscaled_height = unscaled_height;
        message.unscaled_width = unscaled_width;
        message.offset = offset;

        message.jpeg = jpeg_pkt->buf->data;
        message.jpeg_size = jpeg_pkt->buf->size;

        send_scaled_frame_message(&message, height);
        return 0;
}

int send_full_jpeg(AVPacket *jpeg_pkt, const int64_t offset, const int unscaled_width, const int unscaled_height)
{
        struct FrameMessage message;
        message.unscaled_height = unscaled_height;
        message.unscaled_width = unscaled_width;
        message.offset = offset;

        message.jpeg = jpeg_pkt->buf->data;
        message.jpeg_size = jpeg_pkt->buf->size;

        send_frame_message(&message);
        return 0;
}

int send_scaled_frame(AVFrame *frame, const int64_t offset, const int width, const int height)
{
        AVPacket jpeg_pkt;
        av_init_packet(&jpeg_pkt);

        AVFrame *scaledFrame = NULL;
        scaledFrame = scale_frame(frame, width, height);

        // jpeg encode frame
        encode_jpeg(scaledFrame, &jpeg_pkt);

        send_scaled_jpeg(&jpeg_pkt, offset, width, height, frame->width, frame->height);

        av_freep(&(scaledFrame->data));
        av_frame_free(&scaledFrame);
        av_packet_unref(&jpeg_pkt);
        return 0;
}

int send_full_frame(AVFrame *frame, const int64_t offset)
{
        AVPacket jpeg_pkt;
        av_init_packet(&jpeg_pkt);

        AVFrame *scaledFrame = NULL;
        scaledFrame = scale_frame(frame, frame->width, frame->height);

        // jpeg encode frame
        encode_jpeg(scaledFrame, &jpeg_pkt);

        send_full_jpeg(&jpeg_pkt, offset, frame->width, frame->height);

        av_freep(&(scaledFrame->data));
        av_frame_free(&scaledFrame);
        av_packet_unref(&jpeg_pkt);
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
                out->first_pts = pkt->pts;
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

int init_jpeg_encoder(struct in_context *in,
											int width,
											int height,
											const char* codec_name)
{
				int ret = 1;

				if (!(in->encoder_codec = avcodec_find_encoder_by_name(codec_name))) {
								fprintf(stderr, "Could not find encoder '%s'\n", codec_name);

								goto error;
				}

				// Initialize the scaled jpeg context
				if (!(in->encoder_scaled_ccx = avcodec_alloc_context3(in->encoder_codec))) {
								goto error;
				}
				// we need a frame so that we can initialize the encoder's codec
				//in->encoder_ccx->hw_frames_ctx = av_buffer_ref(in->ccx->hw_frames_ctx);
				in->encoder_scaled_ccx->pix_fmt = in->ccx->pix_fmt;
				in->encoder_scaled_ccx->time_base = (AVRational){1, 25}; // unused
				in->encoder_scaled_ccx->width = width;
				in->encoder_scaled_ccx->height = height;

				if ((ret = avcodec_open2(in->encoder_scaled_ccx, in->encoder_codec, NULL)) < 0) {
								fprintf(stderr, "Failed to open encode codec. Error code: %s\n",
												av_err2str(ret));
								goto error;
				}

				// Initialize the full jpeg context
				if (!(in->encoder_ccx = avcodec_alloc_context3(in->encoder_codec))) {
								goto error;
				}
				// we need a frame so that we can initialize the encoder's codec
				in->encoder_ccx->hw_frames_ctx = av_buffer_ref(in->ccx->hw_frames_ctx);
				in->encoder_ccx->pix_fmt = in->ccx->pix_fmt;
				in->encoder_ccx->time_base = (AVRational){1, 25}; // unused
				in->encoder_ccx->width = in->scaled_width;
				in->encoder_ccx->height = in->scaled_height;

				if ((ret = avcodec_open2(in->encoder_ccx, in->encoder_codec, NULL)) < 0) {
								fprintf(stderr, "Failed to open encode codec. Error code: %s\n",
												av_err2str(ret));
								goto error;
				}

        ret = 0;
				error:
				return ret;

}

int push_frame(struct CameraState *cam)
{
        struct in_context *in = &cam->in;
        struct out_context *out = &cam->out;
        AVPacket *pkt = &cam->pkt;
        char filter_str[500];

        // Make sure packet is video
        if (pkt->stream_index != in->stream_index) {
                return 0;
        }
        AVFrame *frame = av_frame_alloc();
        AVFrame *sw_frame = NULL;

        record_metric_start(cam, DECODE_TIME);
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
                bs_log("Error receiving frame: %s", errbuf);
                goto cleanup;
        }
        record_metric_end(cam, DECODE_TIME);
        // Calculate microsecond offset from frame->pts

        const int64_t offset_microseconds = av_rescale_q(frame->pts - out->first_pts,
                                                         in->st->time_base,
                                                         microsecond);

        // Aquire jpeg frame

        // Hardware jpeg encode
        if (in->hw_accel_type == AV_HWDEVICE_TYPE_QSV) {
                int ret = 0;
                if (!in->encoder_initialized) {
												if (init_jpeg_encoder(in, frame->width, frame->height, "mjpeg_qsv") != 0) {
																goto error;
												}
                        in->encoder_initialized = 1;
                }

                AVPacket enc_pkt;
                av_init_packet(&enc_pkt);
                enc_pkt.data = NULL;
                enc_pkt.size = 0;

                // Send scaled frame
                if ((ret = avcodec_send_frame(in->encoder_scaled_ccx, frame)) < 0) {
                        fprintf(stderr, "Error code: %s\n", av_err2str(ret));
                        goto error;
                }
                while (1) {
                        ret = avcodec_receive_packet(in->encoder_scaled_ccx, &enc_pkt);
                        if (ret)
                                break;
                        enc_pkt.stream_index = 0;
                        send_scaled_jpeg(&enc_pkt, offset_microseconds, in->encoder_ccx->width, in->encoder_ccx->height, frame->width, frame->height);
                        av_packet_unref(&enc_pkt);
                }

                // Send full frame
                if ((ret = avcodec_send_frame(in->encoder_ccx, frame)) < 0) {
                        fprintf(stderr, "Error code: %s\n", av_err2str(ret));
                        goto error;
                }
                while (1) {
                        ret = avcodec_receive_packet(in->encoder_ccx, &enc_pkt);
                        if (ret)
                                break;
                        enc_pkt.stream_index = 0;
                        send_full_jpeg(&enc_pkt, offset_microseconds, frame->width, frame->height);
                        av_packet_unref(&enc_pkt);
                }
        } else if (in->hw_accel_type == AV_HWDEVICE_TYPE_VAAPI) {
                int ret = 0;
                if (!in->encoder_initialized) {
												if (init_jpeg_encoder(in, frame->width, frame->height, "mjpeg_vaapi") != 0) {
																goto error;
												}

                        snprintf(filter_str, sizeof(filter_str), "format=vaapi,scale_vaapi=w=%d:h=%d",
                                 in->scaled_width, in->scaled_height);
                        ex_init_input_filters(in, filter_str);

                        in->encoder_initialized = 1;
                }
                AVPacket enc_pkt;
                av_init_packet(&enc_pkt);
                enc_pkt.data = NULL;
                enc_pkt.size = 0;

                // Send scaled frame

                // push frame into filter graph
                ret = av_buffersrc_add_frame_flags(in->buffersrc_ctx, frame, AV_BUFFERSRC_FLAG_KEEP_REF | AV_BUFFERSRC_FLAG_PUSH);
                if (ret < 0) {
                        fprintf(stderr, "Error pushing frame into filter graph %s\n", av_err2str(ret));
                        goto error;
                }
                int ret2;
                AVFrame *fl_frame = av_frame_alloc();
                while ((ret = av_buffersink_get_frame(in->buffersink_ctx, fl_frame)) >= 0) {
												if ((ret2 = avcodec_send_frame(in->encoder_scaled_ccx, fl_frame)) < 0) {
																fprintf(stderr, "Error code: %s\n", av_err2str(ret));
																av_frame_unref(fl_frame);
																ret = ret2;
																goto error;
												}

												while (1) {
																ret2 = avcodec_receive_packet(in->encoder_scaled_ccx, &enc_pkt);
																if (ret2) {
																				av_packet_unref(&enc_pkt);
																				av_frame_unref(fl_frame);
																				break;
																}
																enc_pkt.stream_index = 0;
																send_scaled_jpeg(&enc_pkt, offset_microseconds, in->encoder_ccx->width, in->encoder_ccx->height, frame->width, frame->height);
																av_packet_unref(&enc_pkt);

												}
                }
                av_frame_unref(fl_frame);
                av_frame_free(&fl_frame);

                // Send full frame
                if ((ret = avcodec_send_frame(in->encoder_ccx, frame)) < 0) {
                        fprintf(stderr, "Error code: %s\n", av_err2str(ret));
                        goto error;
                }
                while (1) {
                        ret = avcodec_receive_packet(in->encoder_ccx, &enc_pkt);
                        if (ret)
                                break;
                        enc_pkt.stream_index = 0;
                        send_full_jpeg(&enc_pkt, offset_microseconds, frame->width, frame->height);
                        av_packet_unref(&enc_pkt);
                }

        } else {
                // software jpeg encode
                // retrieve data from gpu, if necessary
                if (is_hwaccel_pix_fmt(frame->format)) {
                        sw_frame = av_frame_alloc();
                        if (av_hwframe_transfer_data(sw_frame, frame, 0) < 0) {
                                fprintf(stderr, "Error transferring the data to system memory\n");
                                goto cleanup;
                        }
                        record_metric_start(cam, JPEG_FULL);
                        send_full_frame(sw_frame, offset_microseconds);
                        record_metric_end(cam, JPEG_FULL);

                        record_metric_start(cam, JPEG_SCALED);
                        send_scaled_frame(sw_frame, offset_microseconds, in->scaled_width, in->scaled_height);
                        record_metric_end(cam, JPEG_SCALED);
                } else {
                        record_metric_start(cam, JPEG_FULL);
                        send_full_frame(frame, offset_microseconds);
                        record_metric_end(cam, JPEG_FULL);
                        record_metric_start(cam, JPEG_SCALED);
                        send_scaled_frame(frame, offset_microseconds, in->scaled_width, in->scaled_height);
                        record_metric_end(cam, JPEG_SCALED);
                }
        }


cleanup:
        av_frame_free(&sw_frame);
        av_frame_free(&frame);
        return 0;
error:
        av_frame_free(&sw_frame);
        av_frame_free(&frame);
        return 1;
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
                bs_log("Error opening stream, %s, error %d!", input_uri, open_ret);
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
                                bs_log("Handle error!");
                                goto cleanup;
                        }
                        if (cam.out.fcx == NULL) {
                                bs_log("FCX null???");
                                continue;
                        }

                        ex_send_packet(&cam.in, &cam.pkt);
                        int write_ret = ex_write_output_packet(&cam.out, cam.in.st->time_base, &cam.pkt);
                        if (write_ret != 0) {
                                bs_log("Write Error!");
                                goto cleanup;
                        }
                        clock_gettime(CLOCK_MONOTONIC, &(cam.in.last_frame_time));
                        record_metric_end(&cam, LOOP_TIME);
                } else {
                        // handle non-video ?
                }
                av_packet_unref(&cam.pkt);
                av_init_packet(&cam.pkt);
                increment_metrics(&cam);
        }

        char buf[1024];
        av_strerror(ret, buf, sizeof(buf));
        bs_log("av_read_frame returned %d, %s exiting...", ret, buf);

cleanup:
        bs_log("Cleaning up!");
        ex_free_input(&cam.in);

        avformat_network_deinit();

        return return_value;
}
