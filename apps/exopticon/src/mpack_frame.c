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

#include <endian.h>
#include <stdio.h>
#include <stdint.h>

#include "mpack.h"

#include "mpack_frame.h"

void send_frame_message(struct FrameMessage *msg)
{
        char *data = NULL;
        size_t size = 0;
        mpack_writer_t writer;
        mpack_writer_init_growable(&writer, &data, &size);

        mpack_start_map(&writer, 2);
        mpack_write_cstr(&writer, "jpegFrame");
        mpack_write_bin(&writer, (char*)msg->jpeg, (uint32_t)msg->jpeg_size);
        mpack_write_cstr(&writer, "pts");
        mpack_write_i64(&writer, msg->pts);
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

        mpack_start_map(&writer, 2);
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

        mpack_start_map(&writer, 2);
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
