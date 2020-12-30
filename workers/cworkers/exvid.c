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

#include <poll.h>
#include <time.h>

#include "libavutil/opt.h"
#include "libavutil/pixdesc.h"

#include "exvid.h"

static int64_t timespec_to_ms_interval(const struct timespec beg,
                                const struct timespec end)
{
        const int64_t billion = 1E9;
        const int64_t million = 1E6;

        int64_t begin_time = (beg.tv_sec * billion) + beg.tv_nsec;
        int64_t end_time = (end.tv_sec * billion) + end.tv_nsec;

        return (end_time - begin_time) / million;
}

static char *timespec_to_8601(struct timespec *ts)
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

static int interrupt_cb(void *ctx)
{
        struct in_context *c = (struct in_context*)ctx;
        struct timespec cur;
        clock_gettime(CLOCK_MONOTONIC, &cur);
        int64_t interval = timespec_to_ms_interval(c->last_frame_time,
                                                   cur);

        struct pollfd pfd;
        pfd.fd = 0;
        pfd.events = 0;
        poll(&pfd, 1, 0);

        int eof = pfd.revents & POLLHUP;

        /*
           if the interval is greater than the timeout or if EOF is
           set on stdin, return 1. Erlang/Elixir set EOF to indicate
           that the process should close.
        */
        return (interval > EX_TIMEOUT_MS || eof);
}

int ex_init(void(*log_callback)(void *, int, const char *, va_list)) {
        // Initialize ffmpeg
        av_log_set_level(AV_LOG_INFO);
        if (log_callback != NULL) {
                av_log_set_callback(log_callback);
        }

        avformat_network_init();

        return 0;
}

int ex_init_input(struct in_context *c) {
        memset(c, 0, sizeof *c);
        return 0;
}

static int hw_decoder_init(struct in_context *c, const enum AVHWDeviceType type) {
    int err = 0;

    if ((err = av_hwdevice_ctx_create(&c->hw_device_ctx, type,
                                      NULL, NULL, 0)) < 0) {
            av_log(NULL, AV_LOG_INFO, "Failed to create specified HW device.");
            return err;
    }

    c->ccx->hw_device_ctx = av_buffer_ref(c->hw_device_ctx);

    return err;
}


static enum AVPixelFormat get_hw_format(AVCodecContext *ctx,
                                        const enum AVPixelFormat *pix_fmts) {
    const enum AVPixelFormat *p;

    // touch ctx to prevent an unused parameter warning
    (void)(ctx);

    for (p = pix_fmts; *p != -1; p++) {
        if (*p == AV_PIX_FMT_CUDA)
            return *p;
    }

    av_log(NULL, AV_LOG_INFO, "Failed to get HW surface format.\n");
    return AV_PIX_FMT_NONE;
}

int ex_open_input_stream(const char *url, struct in_context *c) {
        int return_value = 0;

        c->fcx = avformat_alloc_context();

        // setup interrupt callback
        c->fcx->interrupt_callback.callback = interrupt_cb;
        c->fcx->interrupt_callback.opaque = c;

        // Open input format
        AVDictionary *opts = 0;
        av_dict_set(&opts, "buffer_size", "26214400", 0);
        av_dict_set(&opts, "rtsp_transport", "udp", 0);
        // default reorder queue size is 500
        av_dict_set(&opts, "reorder_queue_size", "2500", 0);
        c->fcx->max_delay = 500000; // 500ms
        clock_gettime(CLOCK_MONOTONIC, &(c->last_frame_time));
        int err = avformat_open_input(&(c->fcx), url, NULL, &opts);
        if (err != 0) {
                char errbuf[100];
                av_strerror(err, errbuf, 100);
                av_log(NULL, AV_LOG_FATAL, "Error opening input file: %s", errbuf);

                // User allocated AVFormatContext is freed on error by
                // avformat_open_input.
                c->fcx = NULL;
                return_value = 1;
                goto cleanup;
        }

        // Probe format for streams
        c->fcx->fps_probe_size = 500;
        clock_gettime(CLOCK_MONOTONIC, &(c->last_frame_time));
        if (avformat_find_stream_info(c->fcx, NULL) < 0) {
                return_value = 2;
                goto cleanup;
        }

        c->stream_index = -1;
        for (uint32_t i = 0; i < c->fcx->nb_streams; i++) {
                c->codecpar = c->fcx->streams[i]->codecpar;
                if (c->codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
                        c->st = c->fcx->streams[i];
                        break;
                }
        }

        clock_gettime(CLOCK_MONOTONIC, &(c->last_frame_time));
        c->stream_index = av_find_best_stream(c->fcx, AVMEDIA_TYPE_VIDEO, -1,
                                              -1, &(c->codec), 0);


        if (c->stream_index < 0) {
                // unable to find video stream
                return_value = 3;
                goto cleanup;
        }
        c->st = c->fcx->streams[c->stream_index];


        // Initialize codec
        c->codec = avcodec_find_decoder(c->codecpar->codec_id);
        if (c->codec == NULL) {
                return_value = 4;
                goto cleanup;
        }


        c->ccx = avcodec_alloc_context3(c->codec);
        if (c->ccx == NULL) {
                return_value = 5;
                goto cleanup;
        }

        if (c->hw_accel_type == AV_HWDEVICE_TYPE_CUDA) {
                for (int i = 0;; i++) {
                        const AVCodecHWConfig *config = avcodec_get_hw_config(c->codec, i);
                        if (!config) {
                                av_log(NULL, AV_LOG_INFO, "Decoder %s does not support device type %s.",
                                       c->codec->name, av_hwdevice_get_type_name(c->hw_accel_type));
                                return_value = 8;
                                goto cleanup;
                        }
                        if (config->methods & AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX &&
                            config->device_type == c->hw_accel_type) {
                                c->hw_pix_fmt = config->pix_fmt;
                                av_log(NULL, AV_LOG_DEBUG, "PIXEL FORMAT: %s", av_get_pix_fmt_name(c->hw_pix_fmt));
                                break;
                        }
                }

                int avcodec_ret = avcodec_parameters_to_context(c->ccx, c->codecpar);
                if (avcodec_ret < 0) {
                        av_log(NULL, AV_LOG_FATAL, "Failed to copy codec pars to context");
                        return_value = 6;
                        goto cleanup;
                }

                c->ccx->get_format = get_hw_format;

                if (hw_decoder_init(c, c->hw_accel_type)) {
                        av_log(NULL, AV_LOG_INFO, "HW decoder successfully initialized!");
                }

                // get formats
                enum AVPixelFormat *hw_formats;
                if (av_hwframe_transfer_get_formats(c->ccx->hw_device_ctx, AV_HWFRAME_TRANSFER_DIRECTION_FROM, &hw_formats, 0) == 0) {
                        for (enum AVPixelFormat *p = hw_formats; *p != AV_PIX_FMT_NONE; p++) {
                                av_log(NULL, AV_LOG_INFO, "HW PIXEL FORMAT: %s", av_get_pix_fmt_name(*p));
                        }
                } else {
                        av_log(NULL, AV_LOG_ERROR, "Failed to fetch hw pixel format");
                        goto cleanup;
                }
        }

        if (avcodec_open2(c->ccx, c->codec, NULL) < 0) {
                av_log(NULL, AV_LOG_FATAL, "Failed to open codec");
                return_value = 7;
                goto cleanup;
        }

        return return_value;
cleanup:
        return return_value;
}

int ex_read_frame(struct in_context *c, AVPacket *pkt)
{
        return av_read_frame(c->fcx, pkt);
}

int ex_send_packet(struct in_context *c, AVPacket *pkt)
{
        int return_value = 0;

        const int d_ret = avcodec_send_packet(c->ccx, pkt);
        if (d_ret != 0) {
                return_value = 1;
        }

        return return_value;
}

int ex_receive_frame(struct in_context *c, AVFrame *frame)
{
        const int receive_ret = avcodec_receive_frame(c->ccx, frame);
        return receive_ret;
}

int ex_free_input(struct in_context *c)
{
        if (c->ccx != NULL) {
                avcodec_free_context(&c->ccx);
        }

        if (c->fcx != NULL) {
                avformat_free_context(c->fcx);
        }
        av_buffer_unref(&c->hw_device_ctx);

        return 0;
}

int ex_init_output(struct out_context *c)
{
        memset(c, 0, sizeof *c);
        c->first_pts = -1;
        c->prev_pts = -1;
        c->size = 0;
        return 0;
}

int ex_open_output_stream(struct in_context *in,
                          struct out_context *out,
                          const char *filename)
{
        int return_value = 0;

        out->size = 0;

        out->fmt = av_guess_format(NULL, filename, NULL);
        avformat_alloc_output_context2(&out->fcx, NULL, NULL, filename);
        if (out->fcx == NULL) {
                return_value = 1;
                goto cleanup;
        }

        // Set avformat options
        av_opt_set(out->fcx, "fflags", "+flush_packets", 0);
        av_opt_set(out->fcx, "avioflags", "+direct", 0);

        for (unsigned int i = 0; i < in->fcx->nb_streams; i++) {
                if (i == (uint64_t)in->stream_index) {
                        AVStream *out_stream;
                        AVStream *in_stream = in->fcx->streams[i];
                        AVCodecParameters *in_codecpar = in_stream->codecpar;
                        out_stream = avformat_new_stream(out->fcx, NULL);
                        if (!out_stream) {
                                fprintf(stderr, "Failed allocating output stream\n");
                                return_value = 2;
                                goto cleanup;
                        }
                        int copy_ret = avcodec_parameters_copy(out_stream->codecpar, in_codecpar);
                        if (copy_ret < 0) {
                                return_value = 3;
                                goto cleanup;
                        }
                        out_stream->codecpar->codec_tag = 0;

//                        out_stream->codec->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
                        out_stream->time_base = in_stream->time_base;
                }
        }

        int ret = avio_open(&out->fcx->pb, filename, AVIO_FLAG_WRITE);
        if (ret < 0) {
                return_value = 4;
                goto cleanup;
        }

        // Set file begin time
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        char *timestring = timespec_to_8601(&ts);

        av_dict_set(&(out->fcx->metadata), "ENDTIME", timestring, 0);
        av_dict_set(&(out->fcx->metadata), "BEGINTIME", timestring, 0);

        free(timestring);

        int header_ret = avformat_write_header(out->fcx, NULL);
        if (header_ret < 0) {
                return_value = 5;
        }

        out->stream_index = in->stream_index;
        strncpy(out->output_path, filename, sizeof out->output_path);

cleanup:
        return return_value;

}

static int64_t find_string_in_file(FILE *file, char *string)
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

int ex_close_output_stream(struct out_context *c)
{
        int ret = 0;
        char *end_time = NULL;
        char filename[1024];
        FILE *output_file = NULL;
        struct timespec end_timestamp;

        ret = clock_gettime(CLOCK_REALTIME, &end_timestamp);
        if (ret == -1) {
                // error!
        }

        end_time = timespec_to_8601(&end_timestamp);

        strncpy(filename, c->fcx->url, sizeof(filename));
        filename[sizeof(filename) - 1] = '\0';

        av_write_trailer(c->fcx);
        avio_closep(&c->fcx->pb);
        avformat_free_context(c->fcx);
        c->fcx = NULL;

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
        // report_finished_file(filename, cam->end_time);

cleanup:
        if (output_file != NULL) {
                fclose(output_file);
        }
        free(end_time);

        return ret;
}

 int ex_write_output_packet(struct out_context *c,
                           AVRational time_base,
                           AVPacket *pkt)
{
        int return_value = 0;
        AVStream *out_stream = c->fcx->streams[pkt->stream_index];

        if (pkt->stream_index != c->stream_index) {
                return 0;
        }

        if (pkt->pts < 0 || (c->prev_pts > 0 && pkt->pts < c->prev_pts)) {
                return 0;
        }
        c->prev_pts = pkt->pts;

        if (c->first_pts == -1) {
                c->first_pts = pkt->pts;
        }

        pkt->pts -= c->first_pts;
        pkt->pts = av_rescale_q_rnd(pkt->pts,
                                    time_base,
                                    out_stream->time_base,
                                    AV_ROUND_NEAR_INF|AV_ROUND_PASS_MINMAX);
        pkt->dts = pkt->pts;
        pkt->duration = av_rescale_q(pkt->duration,
                                     time_base,
                                     out_stream->time_base);


        c->size += pkt->size;
        return_value = av_write_frame(c->fcx, pkt);
        if (return_value < 0) {
                char errbuf[100];
                av_strerror(return_value, errbuf, 100);
                fprintf(stderr, "%s, %d\n", errbuf, return_value);

        }
        av_write_frame(c->fcx, NULL);

        return return_value;
}
