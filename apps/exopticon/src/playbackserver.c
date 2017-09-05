/*
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
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "mpack_frame.h"

#define BILLION 1000000000

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
        const int32_t msg_length = strlen(buf);
        const int32_t be_msg_length = htonl(msg_length);
        fwrite(&be_msg_length, sizeof be_msg_length, 1, stderr);
        fwrite(buf, 1, msg_length, stderr);
        fflush(stderr);

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
        if (avcodec_open2(ccx, codec, NULL) < 0) {
                bs_log("Failed to open codec");
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

        //        int encode_ret = avcodec_encode_video2(ccx, pkt, frame, &got_frame);
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
        avcodec_close(ccx);
        av_free(ccx);
        return ret;
}

int send_frame(const AVFrame *frame, struct timespec begin_time, const AVRational time_base)
{
        AVPacket jpeg_packet;
        av_init_packet(&jpeg_packet);
        encode_jpeg(frame, &jpeg_packet);
        struct FrameMessage message;
        message.jpeg = jpeg_packet.buf->data;
        message.jpeg_size = jpeg_packet.buf->size;
        message.pts = frame->pts;

        do {
                AVRational nsec = av_make_q(1, BILLION);
                struct timespec pts_ts;
                pts_ts.tv_sec = (frame->pts * time_base.num) / time_base.den;
                int64_t frac_sec =
                    frame->pts - ((pts_ts.tv_sec * time_base.den) / time_base.num);
                pts_ts.tv_nsec =
                    av_rescale_q(frac_sec, time_base, nsec);

                struct timespec cur_time;
                clock_gettime(CLOCK_MONOTONIC, &cur_time);
                struct timespec diff = time_diff(begin_time, cur_time);
                if (time_cmp(diff, pts_ts) > -1) {
                        break;
                } else {
                  //usleep(diff.tv_nsec / 1000 / 10);
                  nanosleep(&diff, NULL);
                }

        } while (1);
        send_frame_message(&message);
        return 0;
}

int handle_packet(struct PlayerState *state)
{
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
                if (receive_ret == AVERROR(EINVAL)) {
                        bs_log("Error decoding packet!");
                        goto cleanup;
                }

                send_frame(state->frame, state->begin_time, state->st->time_base);

        } while (receive_ret != AVERROR(EOF));

        return 0;

cleanup:
        return 1;
}

int main(int argc, char *argv[])
{
        struct PlayerState player;

        const char *program_name = argv[0];
        if (argc < 3) {
          fprintf(stderr, "USAGE: ./%s <input filename> <offset in ms>\n", program_name);
        }

        // Initialize ffmpeg
        av_log_set_level(AV_LOG_FATAL);
        av_register_all();
        avcodec_register_all();
        avformat_network_init();

        player.fcx = avformat_alloc_context();
        player.frame = av_frame_alloc();
        player.begin_time.tv_sec = 0;
        player.begin_time.tv_nsec = 0;

        // Determine offset
        long long ms_offset = 0;
        sscanf(argv[2], "%lld", &ms_offset);

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

        //        player.ccx = avcodec_alloc_context3(player.codec;)
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
        while ((ret = av_read_frame(player.fcx, &player.pkt)) >= 0) {
                // handle packet
                handle_packet(&player);
                av_packet_unref(&player.pkt);
                av_init_packet(&player.pkt);
                count++;
        }
        bs_log("Frame count! %d\n", count);

cleanup:
        if (player.fcx != NULL) {
                //                close_output_file(&player);
        }

        //        av_frame_free(&player.frame);

        if (player.fcx != NULL) {
                avformat_close_input(&player.fcx);
        }

        avformat_network_deinit();

        return 0;
}
