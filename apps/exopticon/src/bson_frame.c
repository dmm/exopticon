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
#include <string.h>

#include "bson_frame.h"

// check out http://bsonspec.org/spec.html to make sense of this
void send_frame_message(struct FrameMessage *msg, FILE *stream)
{
        const char int64_tag = 0x12;
        const char binary_tag = 0x05;
        const char *jpeg_name = "frameJpeg";
        const char *pts_name = "pts";

        uint32_t msg_size = 0;
        //
        // Document total and ending null
        //
        msg_size += sizeof(msg_size) + 1;

        //
        // jpeg element
        //
        // jpeg element tag
        msg_size += 1;
        // jpeg element name + ending null
        msg_size += strlen(jpeg_name) + 1;
        // jpeg element int32
        msg_size += sizeof(int32_t);
        // jpeg element subtype byte
        msg_size += 1;
        // jpeg element size
        msg_size += msg->jpeg_size;

        //
        // pts element
        //
        // pts element tag
        msg_size += 1;
        // pts element name + ending null
        msg_size += strlen(pts_name) + 1;
        // pts element
        msg_size += sizeof msg->pts;

        //
        // output framing length
        //
        const uint32_t msg_size_be = htobe32(msg_size);
        fwrite(&msg_size_be, sizeof msg_size_be, 1, stdout);
        //
        // Generate bson
        //
        const char null = 0x00;
        // total message size
        const uint32_t msg_size_le = htole32(msg_size);
        fwrite(&msg_size_le, sizeof msg_size_le, 1, stdout);

        // pts element
        fwrite(&int64_tag, 1, 1, stdout);
        fprintf(stdout, "%s", pts_name);
        fwrite(&null, 1, 1, stdout);
        fwrite(&msg->pts, sizeof msg->pts, 1, stdout);

        // jpeg element, name
        fwrite(&binary_tag, 1, 1, stdout);
        fprintf(stdout, "%s", jpeg_name);
        fwrite(&null, 1, 1, stdout);
        // jpeg element, size
        const uint32_t jpeg_size_le = htole32(msg->jpeg_size);
        fwrite(&jpeg_size_le, sizeof jpeg_size_le, 1, stdout);
        // jpeg element subtype, generic binary type (null, \x00)
        fwrite(&null, 1, 1, stdout);
        // jpeg element, data
        fwrite(msg->jpeg, 1, msg->jpeg_size, stdout);

        // terminal null
        fwrite(&null, 1, 1, stdout);
        fflush(stdout);
}

int32_t get_string_element_size(const char *string, const char *tagname)
{
        int32_t element_size = 0;

        // string element tag
        element_size += 1;

        // tagname and ending null
        element_size += strlen(tagname) + 1;

        // string length prefix
        element_size += sizeof(int32_t);

        // string and ending null
        element_size += strlen(string) + 1;

        return element_size;
}

void write_string(const char* string, const char* tag_name)
{
        const char string_tag = 0x02;
        const char null = 0x0;

        fwrite(&string_tag, 1, 1, stdout);
        fprintf(stdout, "%s", tag_name);
        fwrite(&null, 1, 1, stdout);
        int32_t string_size = htole32(strlen(string) + 1);
        fwrite(&string_size, sizeof string_size, 1, stdout);
        fwrite(string, strlen(string), 1, stdout);
        fwrite(&null, 1, 1, stdout);
}

void send_new_file_message(char *filename, char *iso_begin_time)
{
        const char *type_tagname = "type";
        const char *type = "newFile";
        const char *file_tagname = "filename";
        const char *begin_time_tagname = "beginTime";

        uint32_t msg_size = 0;
        //
        // Document total and ending null
        //
        msg_size += sizeof(msg_size) + 1;

        //
        // type element
        //
        msg_size += get_string_element_size(type, type_tagname);

        //
        // filename element
        msg_size += get_string_element_size(filename, file_tagname);

        //
        // begin time element
        //
        msg_size += get_string_element_size(iso_begin_time, begin_time_tagname);

        //
        // output framing length
        //
        const uint32_t msg_size_be = htobe32(msg_size);
        fwrite(&msg_size_be, sizeof msg_size_be, 1, stdout);

        //
        // Generate bson
        //
        const char null = 0x00;
        // total message size
        const uint32_t msg_size_le = htole32(msg_size);
        fwrite(&msg_size_le, sizeof msg_size_le, 1, stdout);

        // type element
        write_string(type, type_tagname);

        // filename element
        write_string(filename, file_tagname);

        // begin time element
        write_string(iso_begin_time, begin_time_tagname);

        fwrite(&null, 1, 1, stdout);
        fflush(stdout);
}
