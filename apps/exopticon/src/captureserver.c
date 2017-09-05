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

#include "mpack_frame.h"

const int MAX_FILE_SIZE = 10 * 1024 * 1024;
const int CAPTURE_PERFORMANCE_SIZE = 50;

int64_t timespec_to_ms(const struct timespec time);
char *timespec_to_8601(struct timespec *ts);

struct CameraState {
        time_t timenow, timestart;
        int got_key_frame;

        AVFormatContext *ifcx;
        AVInputFormat *ifmt;
        AVCodecContext *iccx;
        AVCodec *icodec;
        AVStream *ist;
        int i_index;

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
        int64_t last_pts;

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

static void my_av_log_callback(void *avcl, int level, const char *fmt,
                               va_list vl)
{
        char output_message[2048];

        if (av_log_get_level() < level) {
                return;
        }

        vsnprintf(output_message, sizeof(output_message), fmt, vl);
        output_message[(sizeof output_message) - 1] =
            '\0'; // I don't remember if vsnprintf always sets this...
        bs_print(
            stderr,
            "{ \"type\": \"message\", \"level\": \"%d\", \"value\": \"%s\" }",
            level, output_message);
        return;
}

static void report_timings(struct FrameTime time)
{
        bs_print("{ \"type\": \"report\", "
                 "\"decodeTime\": %lld, "
                 "\"fileWriteTime\": %lld, "
                 "\"streamWriteTime\": %lld, "
                 "\"readTime\": %lld, "
                 "\"wholeLoop\": %lld "
                 "}",
                 (long long)time.decode_time, (long long)time.file_write_time,
                 (long long)time.stream_write_time, (long long)time.read_time,
                 (long long)time.whole_loop);

        return;
}

void bs_log(const char *const fmt, ...)
{
        va_list ap;
        va_start(ap, fmt);
        //        my_av_log_callback(NULL, 1, fmt, ap);
}

void report_new_file(char *filename, struct timespec begin_time)
{
        char *isotime = timespec_to_8601(&begin_time);
        send_new_file_message(filename, isotime);
        free(isotime);
}
void report_finished_file(char *filename, struct timespec begin_time,
                          struct timespec end_time)
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

int encode_jpeg(AVCodecContext *in_ccx, AVFrame *frame, AVPacket *pkt)
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

        ccx->bit_rate = in_ccx->bit_rate;
        ccx->width = in_ccx->width;
        ccx->height = in_ccx->height;
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
        avcodec_copy_context(cam->ost->codec, cam->iccx);

        cam->ost->sample_aspect_ratio.num = cam->iccx->sample_aspect_ratio.num;
        cam->ost->sample_aspect_ratio.den = cam->iccx->sample_aspect_ratio.den;

        // Assume r_frame_rate is accurate
        bs_log("AVStream time_base: %d / %d, Framerate: %d / %d",
               cam->ist->time_base.num, cam->ist->time_base.den,
               cam->ist->r_frame_rate.num, cam->ist->r_frame_rate.den);
        bs_log("cam->ist ticks_per_fram: %d", cam->ist->codec->ticks_per_frame);

        AVRational output_fps;
        output_fps.num = cam->frames_per_second;
        output_fps.den = 1;
        AVRational output_timebase;
        output_timebase.num = 1;
        output_timebase.den = 1000;
        cam->ost->avg_frame_rate = output_fps;
        // cam->ost->time_base = av_inv_q(cam->ost->r_frame_rate);
        cam->ost->time_base = output_timebase;

        // The cam->ost time_base is 1/fps, so codec->ticks_per_frame should be
        // one. Otherwise the video plays at twice the normal speed.
        //      cam->ost->codec->ticks_per_frame = 1;
        bs_log("cam->ost ticks_per_frame: %d ",
               cam->ost->codec->ticks_per_frame);

        // Set global headers
        cam->ost->codec->flags |= CODEC_FLAG_GLOBAL_HEADER;

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
        report_finished_file(filename, cam->begin_time, cam->end_time);

cleanup:
        if (output_file != NULL) {
                fclose(output_file);
        }
        free(end_time);

        return ret;
}

int64_t timespec_to_ms(const struct timespec time)
{
        const int64_t billion = 1E9;
        const int64_t million = 1E6;

        const int64_t time_ms = time.tv_sec * 1000 + (time.tv_nsec / million);

        return time_ms;
}

int64_t timespec_to_ms_interval(const struct timespec beg,
                                const struct timespec end)
{
        const int64_t billion = 1E9;
        const int64_t million = 1E6;

        int64_t begin_time = (beg.tv_sec * billion) + beg.tv_nsec;
        int64_t end_time = (end.tv_sec * billion) + end.tv_nsec;

        return (end_time - begin_time) / million;
}

int handle_packet(struct CameraState *cam, struct FrameTime *frame_time)
{
        // Make sure packet is video
        if (cam->pkt.stream_index != cam->i_index) {
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

                av_dump_format(cam->ofcx, 0, cam->ofcx->filename, 1);
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

        // Don't trust any of the stream timing information. Instead reconstruct
        // it from the given fps.
        cam->pkt.stream_index = cam->ost->id;
        cam->pkt.pts = cam->last_pts;
        cam->pkt.dts = cam->pkt.pts;
        cam->last_pts += av_rescale_q(1, av_inv_q(cam->ost->avg_frame_rate),
                                      cam->ost->time_base);

        AVPacket decode_packet;
        struct timespec write_begin;
        struct timespec write_end;
        cam->file_size += cam->pkt.size;
        av_init_packet(&decode_packet);
        av_packet_ref(&decode_packet, &(cam->pkt));
        clock_gettime(CLOCK_MONOTONIC, &write_begin);
        av_interleaved_write_frame(cam->ofcx, &cam->pkt);

        clock_gettime(CLOCK_MONOTONIC, &write_end);
        frame_time->file_write_time =
            timespec_to_ms_interval(write_begin, write_end);

        AVPacket jpegPkt;
        av_init_packet(&jpegPkt);
        uint8_t *output_frame_buffer;
        int output_frame_size;
        int got_frame = 0;
        struct FrameMessage message;
        struct timespec decode_begin;
        struct timespec decode_end;
        clock_gettime(CLOCK_MONOTONIC, &decode_begin);
        int len = avcodec_decode_video2(cam->iccx, cam->frame, &got_frame,
                                        &decode_packet);
        clock_gettime(CLOCK_MONOTONIC, &decode_end);
        frame_time->decode_time =
            timespec_to_ms_interval(decode_begin, decode_end);

        if (len <= 0 || got_frame == 0) {
                bs_log("Error decoding frame! len %d, got_frame: %d", len,
                       got_frame);
                goto cleanup;
        }

        if (cam->icodec->id == AV_CODEC_ID_MJPEG ||
            cam->icodec->id == AV_CODEC_ID_MJPEGB) {
                // The video codec is jpeg so we just use the encoded frame
                // as-is.
                output_frame_size = decode_packet.buf->size;
                output_frame_buffer = decode_packet.buf->data;

                message.jpeg = decode_packet.buf->data;
                message.jpeg_size = decode_packet.buf->size;
        } else {
                // The video codec is not jpeg so we need to encode a jpeg
                // frame.
                encode_jpeg(cam->iccx, cam->frame, &jpegPkt);
                output_frame_size = jpegPkt.buf->size;
                output_frame_buffer = jpegPkt.buf->data;

                message.jpeg = jpegPkt.buf->data;
                message.jpeg_size = jpegPkt.buf->size;
        }
        message.pts = cam->frame->pts;

        clock_gettime(CLOCK_MONOTONIC, &write_begin);
        /*
        // Write output image to stdout, prefixed with big-endian length
        uint32_t be_pkt_size = htonl(output_frame_size);
        fwrite(&be_pkt_size, sizeof(be_pkt_size), 1, stdout);
        fwrite(output_frame_buffer, output_frame_size, 1, stdout);
        */
        send_frame_message(&message);
        clock_gettime(CLOCK_MONOTONIC, &write_end);
        frame_time->stream_write_time =
            timespec_to_ms_interval(write_begin, write_end);

cleanup:
        av_packet_unref(&jpegPkt);
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

static int interrupt_cb(void *ctx)
{
        struct CameraState *cam = (struct CameraState *)ctx;
        const int64_t timeout = 5000;
        struct timespec cur;
        const struct timespec beg = cam->frame_begin_time;
        clock_gettime(CLOCK_MONOTONIC, &cur);

        // If tv_sec is zero, capture hasn't started so don't kill anything.
        const int64_t interval =
            beg.tv_sec ? timespec_to_ms_interval(beg, cur) : 0;

        return (interval > timeout) || checkforquit();
}

int main(int argc, char *argv[])
{
        int return_value = EXIT_SUCCESS;
        struct CameraState cam;
        cam.ifcx = NULL;
        cam.ofcx = NULL;
        cam.frame = NULL;

        const char *program_name = argv[0];
        const char *input_uri;
        const char *output_directory_name;

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
        av_log_set_level(AV_LOG_FATAL);
        av_register_all();
        avcodec_register_all();
        avformat_network_init();

        cam.ifcx = NULL;
        cam.ofcx = NULL;
        cam.got_key_frame = 0;
        cam.frame = av_frame_alloc();
        cam.file_size = 0;

        //
        // Input
        //
        // Allocated input AVFormatContext so we can set the i/o
        // callback prior to calling avformat_open_input
        cam.ifcx = avformat_alloc_context();

        /*
         * Initialize i/o callback to implement av_read_frame timeout.
         */
        cam.ifcx->interrupt_callback.callback = interrupt_cb;
        cam.ifcx->interrupt_callback.opaque = &cam;
        cam.frame_begin_time.tv_sec = 0;

        // open rtsp
        AVDictionary *opts = 0;
        av_dict_set(&opts, "rtsp_transport", "tcp", 0);
        if (avformat_open_input(&(cam.ifcx), input_uri, NULL, &opts) != 0) {
                // try udp, reset ifcx because the first call trashes it
                cam.ifcx = avformat_alloc_context();
                cam.ifcx->interrupt_callback.callback = interrupt_cb;
                cam.ifcx->interrupt_callback.opaque = &cam;
                cam.frame_begin_time.tv_sec = 0;

                av_dict_set(&opts, "rtsp_transport", "udp", 0);
                if (avformat_open_input(&(cam.ifcx), input_uri, NULL, &opts) !=
                    0) {

                        bs_print("ERROR: Cannot open input file");
                        // User allocated AVFormatContext is freed on error by
                        // avformat_open_input.
                        cam.ifcx = NULL;
                        return_value = 1;
                        goto cleanup;
                }
        }
        cam.ifcx->fps_probe_size = 500;

        if (avformat_find_stream_info(cam.ifcx, NULL) < 0) {
                bs_log("ERROR: Cannot find stream info");
                goto cleanup;
        }

        snprintf(cam.ifcx->filename, sizeof(cam.ifcx->filename), "%s",
                 input_uri);

        // search video stream
        cam.i_index = -1;
        for (unsigned i = 0; i < cam.ifcx->nb_streams; i++) {
                cam.iccx = cam.ifcx->streams[i]->codec;
                if (cam.iccx->codec_type == AVMEDIA_TYPE_VIDEO) {
                        cam.ist = cam.ifcx->streams[i];
                        cam.i_index = i;
                        break;
                }
        }
        if (cam.i_index < 0) {
                bs_log("ERROR: Cannot find input video stream");
                goto cleanup;
        }

        // Initialize code for decoding
        cam.icodec = avcodec_find_decoder(cam.iccx->codec_id);
        if (!cam.icodec) {
                bs_log("Codec not found %d ", cam.iccx->codec_id);
                return_value = 2;
                goto cleanup;
        }
        if (avcodec_open2(cam.iccx, cam.icodec, NULL) < 0) {
                bs_log("Could not open codec");
                return_value = 2;
                goto cleanup;
        }
        // Assume ist framerate is not accurate :(

        //
        // Output
        // start reading packets from stream and write them to file

        av_dump_format(cam.ifcx, 0, cam.ifcx->filename, 0);

        // av_read_play(context);//play RTSP (Shouldn't need this since it
        // defaults to playing on connect)
        av_init_packet(&cam.pkt);
        cam.switch_file = 1;
        int ret = 0;
        struct CapturePerformance perf;
        perf.count = 0;
        struct FrameTime frame_time;
        memset(&frame_time, 0, sizeof(frame_time));
        struct timespec read_begin, read_end;
        clock_gettime(CLOCK_MONOTONIC, &read_begin);
        while ((ret = av_read_frame(cam.ifcx, &cam.pkt)) >= 0) {
                clock_gettime(CLOCK_MONOTONIC, &read_end);
                handle_packet(&cam, &frame_time);
                frame_time.read_time =
                    timespec_to_ms_interval(read_begin, read_end);

                av_packet_unref(&cam.pkt);
                av_init_packet(&cam.pkt);

                perf.count %= 50;
                perf.frame_times[perf.count] = frame_time;

                if (perf.count == 0 || perf.count == 25) {
                        report_timings(frame_time);
                }

                perf.count++;

                clock_gettime(CLOCK_MONOTONIC, &read_begin);
                frame_time.whole_loop =
                    timespec_to_ms_interval(read_end, read_begin);
                clock_gettime(CLOCK_MONOTONIC, &read_begin);
        }
        char buf[1024];
        av_strerror(ret, buf, sizeof(buf));
        bs_log("av_read_frame returned %d, %s exiting...", ret, buf);

cleanup:
        bs_log("Cleaning up!");
        if (cam.ofcx != NULL) {
                close_output_file(&cam);
        }

        av_frame_free(&cam.frame);
        if (cam.ifcx != NULL) {
                avformat_close_input(&cam.ifcx);
        }

        if (cam.iccx != NULL) {
                //                avcodec_close(cam.iccx);
        }

        avformat_network_deinit();

        return return_value;
}
