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

#include <endian.h>
#include <stdio.h>
#include <stdint.h>

#include <libavutil/log.h>

#include "mpack.h"

#include "mpack_frame.h"

void send_frame_message(struct FrameMessage *msg)
{
        char *data = NULL;
        size_t size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 5);
        mpack_write_cstr(&writer, "type");
        mpack_write_cstr(&writer, "frame");
        mpack_write_cstr(&writer, "jpegFrame");
        mpack_write_bin(&writer, (char*)msg->jpeg, (uint32_t)msg->jpeg_size);
        mpack_write_cstr(&writer, "offset");
        mpack_write_i64(&writer, msg->offset);
        mpack_write_cstr(&writer, "unscaledWidth");
        mpack_write_i32(&writer, msg->unscaled_width);
        mpack_write_cstr(&writer, "unscaledHeight");
        mpack_write_i32(&writer, msg->unscaled_height);
        mpack_finish_map(&writer);

        if (mpack_writer_destroy(&writer) != mpack_ok) {
                exit(5);
        }

        // Write the total message size for framing
        const uint32_t frame_size = htobe32((uint32_t)size);
        fwrite(&frame_size, sizeof(frame_size), 1, stdout);

        // Write message
        fwrite(data, size, 1, stdout);
        fflush(stdout);
        free(data);
}

void send_scaled_frame_message(struct FrameMessage *msg, const int32_t height)
{
        char *data = NULL;
        size_t size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 6);
        mpack_write_cstr(&writer, "type");
        mpack_write_cstr(&writer, "frameScaled");
        mpack_write_cstr(&writer, "jpegFrameScaled");
        mpack_write_bin(&writer, (char*)msg->jpeg, (uint32_t)msg->jpeg_size);
        mpack_write_cstr(&writer, "height");
        mpack_write_i32(&writer, height);
        mpack_write_cstr(&writer, "offset");
        mpack_write_i64(&writer, msg->offset);
        mpack_write_cstr(&writer, "unscaledWidth");
        mpack_write_i32(&writer, msg->unscaled_width);
        mpack_write_cstr(&writer, "unscaledHeight");
        mpack_write_i32(&writer, msg->unscaled_height);
        mpack_finish_map(&writer);

        if (mpack_writer_destroy(&writer) != mpack_ok) {
                exit(5);
        }

        // Write the total message size for framing
        const uint32_t frame_size = htobe32((uint32_t)size);
        fwrite(&frame_size, sizeof(frame_size), 1, stdout);

        // Write message
        fwrite(data, size, 1, stdout);
        fflush(stdout);
        free(data);
}

void send_new_file_message(char *filename, char *iso_begin_time)
{
        char *data = NULL;
        size_t size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 3);
        mpack_write_cstr(&writer, "type");
        mpack_write_cstr(&writer, "newFile");
        mpack_write_cstr(&writer, "filename");
        mpack_write_cstr(&writer, filename);
        mpack_write_cstr(&writer, "beginTime");
        mpack_write_cstr(&writer, iso_begin_time);
        mpack_finish_map(&writer);

        if (mpack_writer_destroy(&writer) != mpack_ok) {
                // Error!
                exit(5);
        }

        // Write the total message size for framing
        const uint32_t frame_size = htobe32((uint32_t)size);
        fwrite(&frame_size, sizeof(frame_size), 1, stdout);

        fwrite(data, size, 1, stdout);
        fflush(stdout);
        free(data);
}

void send_end_file_message(char *filename, char *iso_end_time)
{
        char *data = NULL;
        size_t size = 0;
        uint32_t frame_size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 3);
        mpack_write_cstr(&writer, "type");
        mpack_write_cstr(&writer, "endFile");
        mpack_write_cstr(&writer, "filename");
        mpack_write_cstr(&writer, filename);
        mpack_write_cstr(&writer, "endTime");
        mpack_write_cstr(&writer, iso_end_time);
        mpack_finish_map(&writer);

        if (mpack_writer_destroy(&writer) != mpack_ok) {
                // Error!
                exit(5);
        }

        // Write the total message size for framing
        frame_size = htobe32((uint32_t)size);
        fwrite(&frame_size, sizeof(frame_size), 1, stdout);

        fwrite(data, size, 1, stdout);
        fflush(stdout);
        free(data);
}

char *get_log_level(int level)
{
        switch(level) {
        case AV_LOG_QUIET:
                return "quiet";
                break;
        case AV_LOG_PANIC:
                return "panic";
                break;
        case AV_LOG_FATAL:
                return "fatal";
                break;
        case AV_LOG_ERROR:
                return "error";
                break;
        case AV_LOG_WARNING:
                return "warning";
                break;
        case AV_LOG_INFO:
                return "info";
                break;
        case AV_LOG_DEBUG:
                return "debug";
                break;
        default:
                return "unknown";
        }
        return "unknown";
}

void send_log_message(int level, char *message)
{
        char *data = NULL;
        size_t size = 0;
        uint32_t frame_size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 3);
        mpack_write_cstr(&writer, "type");
        mpack_write_cstr(&writer, "log");
        mpack_write_cstr(&writer, "level");
        mpack_write_cstr(&writer, get_log_level(level));
        mpack_write_cstr(&writer, "message");
        mpack_write_cstr(&writer, message);
        mpack_finish_map(&writer);

        if (mpack_writer_destroy(&writer) != mpack_ok) {
                // Error!
                exit(5);
        }

        // Write the total message size for framing
        frame_size = htobe32((uint32_t)size);
        fwrite(&frame_size, sizeof(frame_size), 1, stdout);
        fwrite(data, size, 1, stdout);
        fflush(stdout);
        free(data);
}
