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

#ifndef EXVID_H
#define EXVID_H

#include <libavcodec/avcodec.h>
#include <libavfilter/buffersink.h>
#include <libavfilter/buffersrc.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <sys/time.h>

#define EX_TIMEOUT_MS 5000

struct in_context {
        AVFormatContext     *fcx;
        AVInputFormat       *fmt;
        AVCodecContext      *ccx;
        AVCodecParameters   *codecpar;
        AVCodec             *codec;
        AVStream            *st;
        int                 stream_index;
        enum AVPixelFormat  hw_pix_fmt;
        enum AVHWDeviceType hw_accel_type;
        AVBufferRef         *hw_device_ctx;

        int scaled_height;
        int scaled_width;

        struct timespec     last_frame_time;
        struct timespec     interrupt_time;

				// only for ex_send_packet
        struct timespec     first_frame_time;
				int64_t             last_pts;
				char                use_walltime_timestamps;
				int64_t             packet_count;

        // Encoder contexts
        int            encoder_initialized;
        AVCodec        *encoder_codec;
        AVCodecContext *encoder_ccx;
        AVCodecContext *encoder_scaled_ccx;

        // Contexts for hwaccel scaling
        AVFilterGraph  *filter_graph;
        AVFilterContext *buffersrc_ctx;
        AVFilterContext *buffersink_ctx;
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
int ex_init_input_filters(struct in_context *c, char *filters_desc);
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
int64_t timespec_to_ms_interval(const struct timespec beg,
                                const struct timespec end);
int ex_send_packet(struct in_context *c, AVPacket *pkt);

#endif // EXVID_H
