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

#ifndef EXOPTICON_TIMING_H
#define EXOPTICON_TIMING_H

enum TIMING {
        DECODE_TIME,
        FILE_WRITE_TIME,
        STREAM_WRITE_TIME,
        READ_TIME,
        LOOP_TIME,
        TIMING_COUNT // keep me at the end
};

struct Interval {
        struct timespec begin;
        struct timespec end;
};

struct FrameTime {
        struct Interval times[TIMING_COUNT];
};

struct CapturePerformance {
        struct FrameTime frame_times[50];
        int count;
};

struct TimingReport {
        int64_t avg;
        int64_t min;
        int64_t max;
};

#endif // EXOPTICON_TIMING_H
