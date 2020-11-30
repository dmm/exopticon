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


#ifndef __MPACK_FRAME_H
#define __MPACK_FRAME_H

struct FrameMessage {
        uint8_t *jpeg;
        int32_t jpeg_size;
        int64_t offset;
        int32_t unscaled_height;
        int32_t unscaled_width;
};

void send_frame_message(struct FrameMessage *msg);
void send_scaled_frame_message(struct FrameMessage *msg, const int32_t height);
void send_new_file_message(char *filename, char *iso_begin_time);
void send_end_file_message(char *filename, char *iso_end_time);
void send_log_message(int level, char *message);

#endif // __MPACK_FRAME_H
