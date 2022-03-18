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

#define _DEFAULT_SOURCE

#include <arpa/inet.h>
#include <endian.h>
#include <fcntl.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavformat/avio.h>
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
#include <libavutil/pixfmt.h>
#include <poll.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "exvid.h"
#include "mpack_frame.h"

#define BILLION 1E9
#define MILLION 1E6

struct PlayerState {
        AVFormatContext *fcx;
        AVInputFormat *fmt;
        AVCodecContext *ccx;
        AVCodecParameters *codecpar;
        AVCodec *codec;
        AVStream *st;

        int32_t i_index;
        AVPacket pkt;

        char got_key_frame;

        AVFrame *frame;
        struct timespec begin_time;
        int64_t first_pts;
        int64_t frame_count;
};

static const AVRational microsecond =
  {
   .num = 1,
   .den = MILLION,
  };

static struct timespec time_diff(struct timespec old_time, struct timespec time)
{
        int64_t sec = time.tv_sec - old_time.tv_sec;
        int64_t nsec = time.tv_nsec - old_time.tv_nsec;
        if (nsec < 0) {
                nsec += BILLION;
                sec -= 1;
        }
        return (struct timespec){.tv_sec = sec, .tv_nsec = nsec};
}
/*
static struct timespec time_since(struct timespec old_time)
{
        struct timespec time;
        clock_gettime(CLOCK_MONOTONIC, &time);
        return time_diff(old_time, time);
}
*/
// return 1 if a > b, -1 if b > a, 0 if a == b
static int time_cmp(struct timespec a, struct timespec b)
{
        struct timespec diff = time_diff(a, b);

        if (diff.tv_sec > 2) {
                return 1;
        }
        if (diff.tv_sec == 0 && diff.tv_nsec == 0) {
                return 0;
        } else if (diff.tv_sec < 0 || diff.tv_nsec < 0) {
                return 1;
        }

        return -1;
}

static pthread_mutex_t log_mutex = PTHREAD_MUTEX_INITIALIZER;

static void bs_print(const char *fmt, ...)
{
        va_list ap;
        char buf[2048];

        buf[0] = 0;
        pthread_mutex_lock(&log_mutex);

        va_start(ap, fmt);
        vsnprintf(buf, sizeof(buf), fmt, ap);
        buf[(sizeof buf) - 1] = '\0';
        send_log_message(1, buf);
        pthread_mutex_unlock(&log_mutex);
}

static void my_av_log_callback(void *avcl, int level, const char *fmt,
                               va_list vl)
{
        char output_message[2048];

        return;
        if (av_log_get_level() < level) {
                return;
        }
        char *a = (char*)avcl;
        vsnprintf(output_message, sizeof(output_message), fmt, vl);
        output_message[(sizeof output_message) - 1] =
            '\0'; // I don't remember if vsnprintf always sets this...
        bs_print(
            "{ \"type\": \"message\", \"level\": \"%d\", \"value\": \"%s\" }\n",
            level, output_message);
        return;
}

void bs_log(const char *const fmt, ...)
{
        va_list ap;
        va_start(ap, fmt);
        my_av_log_callback(NULL, 1, fmt, ap);
}

int encode_jpeg(const AVFrame *in_frame, AVPacket *pkt)
{
        AVCodec *codec = NULL;
        AVFrame *frame = NULL;
        AVCodecContext *ccx = NULL;
        const enum AVPixelFormat img_fmt = AV_PIX_FMT_YUVJ420P;
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

//        ccx->bit_rate = in_ccx->bit_rate;
        ccx->width = in_frame->width;
        ccx->height = in_frame->height;
        ccx->pix_fmt = img_fmt;

        // Set quality
        ccx->qmin = 2;
        ccx->qmax = 10;
        ccx->mb_lmin = ccx->qmin * FF_QP2LAMBDA;
        ccx->mb_lmax = ccx->qmax * FF_QP2LAMBDA;
        ccx->time_base.num = 5;
        ccx->time_base.den = 1;

        pkt->data = NULL;
        pkt->size = 0;
        int codec_open_ret = avcodec_open2(ccx, codec, NULL);
        if (codec_open_ret < 0) {
                bs_log("Failed to open codec");
                char error[256];
                av_strerror(codec_open_ret, error, 256);
                fprintf(stderr, "%s", error);
                ret = 1;
                goto cleanup;
        }

        frame = av_frame_clone(in_frame);
        if (frame == NULL) {
                bs_log("frame clone error!");
                ret = 1;
                goto cleanup;
        }

        frame->pts = 1;
        frame->quality = ccx->global_quality;
        frame->format = img_fmt;
        frame->width = ccx->width;
        frame->height = ccx->height;

        const int encode_ret = avcodec_send_frame(ccx, frame);
        if (encode_ret != 0) {
                bs_log("Error encoding jpeg");
                ret = 1;
                goto cleanup;
        }
        int receive_ret = avcodec_receive_packet(ccx, pkt);
        if (receive_ret != 0) {
          bs_log("Error receiving encoded packet");
          ret = 1;
          goto cleanup;
        }

cleanup:
        av_frame_free(&frame);
        avcodec_close(ccx);
        av_free(ccx);
        return ret;
}

int seek_to_offset(struct PlayerState *state, int64_t ms_offset)
{
        int64_t offset = av_rescale_q(ms_offset,
                                      microsecond,
                                      state->fcx->streams[state->i_index]->time_base);

        int flags = AVSEEK_FLAG_FRAME | AVSEEK_FLAG_BACKWARD;

        av_seek_frame(state->fcx, state->i_index, offset, flags);
        avcodec_flush_buffers(state->ccx);
        return 0;
}

int send_frame(const AVFrame *frame, struct timespec begin_time, int64_t first_pts, const AVRational time_base)
{
        AVPacket jpeg_packet;
        av_init_packet(&jpeg_packet);
        encode_jpeg(frame, &jpeg_packet);
        struct FrameMessage message;
        message.jpeg = jpeg_packet.buf->data;
        message.jpeg_size = jpeg_packet.buf->size;
        message.unscaled_width = frame->width;
        message.unscaled_height = frame->height;
        //        message.pts = frame->pts;
        int64_t pts = frame->pts - first_pts;

        // Calculate offset, in microseconds
        const AVRational microsecond = {
                                        .num = 1,
                                        .den = 1E6,
        };
        message.offset = av_rescale_q(frame->pts, time_base, microsecond);

        do {
                AVRational nsec = av_make_q(1, BILLION);
                struct timespec pts_ts;
                pts_ts.tv_sec = (pts * time_base.num) / time_base.den;
                int64_t frac_sec =
                    pts - ((pts_ts.tv_sec * time_base.den) / time_base.num);
                pts_ts.tv_nsec =
                    av_rescale_q(frac_sec, time_base, nsec);

                struct timespec cur_time;
                clock_gettime(CLOCK_MONOTONIC, &cur_time);

                struct timespec diff = time_diff(begin_time, cur_time);
/*
                fprintf(stderr, "\nframe->pts: %lld, time_base: %lld,%lld\n",
                        frame->pts,
                        time_base.num,
                        time_base.den);

                fprintf(stderr, "Comparing diff: %lld.%lld, pts_ts: %lld.%lld\n",
                        (long long)diff.tv_sec,
                        (long long)diff.tv_nsec,
                        (long long)pts_ts.tv_sec,
                        (long long)pts_ts.tv_nsec);
*/
                if (time_cmp(diff, pts_ts) == 1) {
                        /*
                        fprintf(stderr, "\nPlaying frame!\n");
                        fflush(stderr);
                        */
                        break;
                } else {
                        struct timespec remaining = time_diff(diff, pts_ts);
                        /*
                        fprintf(stderr, "Time remaining: %lld.%lld\n",
                                (long long)remaining.tv_sec,
                                (long long)remaining.tv_nsec);
                        */
                        nanosleep(&remaining, NULL);
                }

        } while (1);

        send_frame_message(&message);
        av_packet_unref(&jpeg_packet);
        return 0;
}

int send_eof()
{
        char *filename = "";
        char *iso_end_time = "1970-01-01T00:00:00Z";
        send_end_file_message(filename, iso_end_time);
        return 0;
}

int handle_packet(struct PlayerState *state, int64_t ms_offset, int playback_rate)
{
        /*
                fprintf(stderr, "\npacket->pts: %lld, packet->dts: %lld, time_base: %lld,%lld\n",
                state->pkt.pts,
                        state->pkt.dts,
                        state->st->time_base.num,
                        state->st->time_base.den
                        );
        */
        // Make sure packet is video
        if (state->pkt.stream_index != state->i_index) {
                return 0;
        }

        // Make sure we start on a key frame
        if (state->got_key_frame == 0 &&
            !(state->pkt.flags & AV_PKT_FLAG_KEY)) {
                return 0;
        }

        if (state->pkt.flags & AV_PKT_FLAG_KEY) {
                // pkt is a keyframe
                state->got_key_frame = 1;
        }


        const int decode_ret = avcodec_send_packet(state->ccx, &state->pkt);
        if (decode_ret != 0) {
                bs_log("Error decoding packet!");
                goto cleanup;
        }

        int receive_ret = 0;
        do {
                receive_ret = avcodec_receive_frame(state->ccx, state->frame);
                if (receive_ret != 0) {
                        bs_log("Error decoding packet!");
                        goto cleanup;
                }

                // Check if we are beyond offset argument, if not,
                // ignore frame.  The earlier call to seek_to_offset
                // should have done most of the work, putting us on
                // the proceeding I frame.
                int64_t offset = av_rescale_q(ms_offset,
                                              microsecond,
                                              state->fcx->streams[state->i_index]->time_base);
                if (state->frame->pts < offset) {
                        continue;
                }
                if (state->first_pts == -1) {
                        state->first_pts = state->frame->pts;
                }
                state->frame_count++;

                // Adjust frame pts by playback_rate
                state->frame->pts = state->frame->pts / playback_rate;

                if (state->frame_count % playback_rate == 0) {
                        send_frame(state->frame, state->begin_time, state->first_pts, state->st->time_base);
                }

        } while (receive_ret != AVERROR(EOF));

        return 0;

cleanup:
        return 1;
}

int checkforquit()
{
        struct pollfd pfd;
        pfd.fd = 0;
        pfd.events = 0;
        poll(&pfd, 1, 0);

        int eof = pfd.revents & POLLHUP;

        return eof;
}

int main(int argc, char *argv[])
{
        struct PlayerState player;

        const char *program_name = argv[0];
        if (argc < 3) {
          fprintf(stderr, "USAGE: ./%s <input filename> <offset in microseconds> (<playback rate>)\n", program_name);
        }

        int playback_rate = 1;

        if (argc > 3) {
                playback_rate = atoi(argv[3]);
        }
        fprintf(stderr, "playback rate: %d\n", playback_rate);
        // Initialize ffmpeg
        av_log_set_level(AV_LOG_FATAL);
        av_register_all();
        avcodec_register_all();
        avformat_network_init();
        // End Initialize ffmpeg

        player.got_key_frame = 0;
        player.fcx = avformat_alloc_context();
        player.frame = av_frame_alloc();
        player.begin_time.tv_sec = 0;
        player.begin_time.tv_nsec = 0;
        player.first_pts = -1;
        player.frame_count = 0;

        // Determine offset
        int64_t ms_offset = 0;
        sscanf(argv[2], "%ld", &ms_offset);

        if (avformat_open_input(&player.fcx, argv[1], NULL, NULL) != 0) {
                bs_log("Could not open input!");
                goto cleanup;
        }

        if (avformat_find_stream_info(player.fcx, NULL) < 0) {
                bs_log("Could not find stream info!");
                goto cleanup;
        }
        bs_log("Searching video stream!");
        // search video stream
        player.i_index = -1;
        for (uint32_t i = 0; i < player.fcx->nb_streams; i++) {
                player.codecpar = player.fcx->streams[i]->codecpar;
                if (player.codecpar->codec_type == AVMEDIA_TYPE_VIDEO) {
                        player.st = player.fcx->streams[i];
                        player.i_index = (int32_t)i;
                        break;
                }
        }
        player.i_index = av_find_best_stream(player.fcx, AVMEDIA_TYPE_VIDEO, -1,
                                             -1, &player.codec, 0);
        if (player.i_index < 0) {
                bs_log("ERROR: Cannot find input video stream");
                goto cleanup;
        }

        player.codec = avcodec_find_decoder(player.codecpar->codec_id);
        if (player.codec == NULL) {
                bs_log("Codec!");
                goto cleanup;
        }

        player.ccx = avcodec_alloc_context3(player.codec);
        if (player.ccx == NULL) {
                bs_log("codec context!");
                goto cleanup;
        }

        int avcodec_ret = avcodec_parameters_to_context(player.ccx, player.codecpar);
        if (avcodec_ret < 0) {
                bs_log("codec create!");
                goto cleanup;
        }

        if (avcodec_open2(player.ccx, player.codec, NULL) < 0) {
                bs_log("PLAYER!");
                goto cleanup;
        }

        // Initialize begin_time
        clock_gettime(CLOCK_MONOTONIC, &player.begin_time);

        int ret = 0;
        int count = 0;
        seek_to_offset(&player, ms_offset);
        while (checkforquit() != 1 && (ret = av_read_frame(player.fcx, &player.pkt)) >= 0) {
                // handle packet
                handle_packet(&player, ms_offset, playback_rate);
                av_packet_unref(&player.pkt);
                av_init_packet(&player.pkt);
                count++;
        }
        send_eof();

cleanup:
        avcodec_close(player.ccx);
        //        avformat_close_input(player.fcx);
        avformat_network_deinit();

        return 0;
}
