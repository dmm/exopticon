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

#ifndef EXVID_H
#define EXVID_H

#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <sys/time.h>

#define EX_TIMEOUT_MS 5000

struct in_context {
        AVFormatContext   *fcx;
        AVInputFormat     *fmt;
        AVCodecContext    *ccx;
        AVCodecParameters *codecpar;
        AVCodec           *codec;
        AVStream          *st;
        int               stream_index;

        struct timespec   last_frame_time;
};

struct out_context {
        AVFormatContext *fcx;
        AVOutputFormat  *fmt;
        AVCodecContext  *ccx;
        AVCodec         *codec;
        char            output_path[500];
        int             stream_index;

        int64_t         first_pts;
        int64_t         prev_pts;
        int64_t         size;
};

int ex_init(void(*)(void *, int, const char *, va_list));

int ex_init_input(struct in_context *context);
int ex_open_input_stream(const char *url, struct in_context *context);
int ex_read_frame(struct in_context *c, AVPacket *pkt);
int ex_free_input(struct in_context *c);

int ex_init_output(struct out_context *context);
int ex_open_output_stream(struct in_context *in,
                          struct out_context *out,
                          const char *filename);
int ex_close_output_stream(struct out_context *c);
int ex_write_output_packet(struct out_context *c,
                           AVRational time_base,
                           AVPacket *pkt);
#endif // EXVID_H
