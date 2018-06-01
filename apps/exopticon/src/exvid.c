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

#include <poll.h>
#include <time.h>

#include "exvid.h"

int64_t timespec_to_ms_interval(const struct timespec beg,
                                const struct timespec end)
{
        const int64_t billion = 1E9;
        const int64_t million = 1E6;

        int64_t begin_time = (beg.tv_sec * billion) + beg.tv_nsec;
        int64_t end_time = (end.tv_sec * billion) + end.tv_nsec;

        return (end_time - begin_time) / million;
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
        av_log_set_level(AV_LOG_DEBUG);
        if (log_callback != NULL) {
                av_log_set_callback(log_callback);
        }
        av_register_all();
        avcodec_register_all();
        avformat_network_init();

        return 0;
}

int ex_open_input_stream(const char *url, struct in_context *c) {
        int return_value = 0;
        c->fcx = avformat_alloc_context();

        // setup interrupt callback
        c->fcx->interrupt_callback.callback = interrupt_cb;
        c->fcx->interrupt_callback.opaque = c;

        // Open input format
        AVDictionary *opts = 0;
        av_dict_set(&opts, "buffer_size", "655360", 0);
        av_dict_set(&opts, "rtsp_transport", "udp", 0);
        clock_gettime(CLOCK_MONOTONIC, &(c->last_frame_time));
        int err = avformat_open_input(&(c->fcx), url, NULL, &opts);
        if (err != 0) {
                char errbuf[100];
                av_strerror(err, errbuf, 100);
                fprintf(stderr, "%s, %d\n", errbuf, err);
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

        int avcodec_ret = avcodec_parameters_to_context(c->ccx, c->codecpar);
        if (avcodec_ret < 0) {
                return_value = 6;
                goto cleanup;
        }

        if (avcodec_open2(c->ccx, c->codec, NULL) < 0) {
                return_value = 7;
                goto cleanup;
        }

        if(avcodec_open2(c->ccx, c->codec, NULL) < 0) {
                return_value = 8;
                goto cleanup;
        }

        return return_value;
cleanup:
        return return_value;
}

int ex_read_frame(struct in_context *c, AVPacket *pkt)
{
        clock_gettime(CLOCK_MONOTONIC, &c->last_frame_time);
        return av_read_frame(c->fcx, pkt);
}
