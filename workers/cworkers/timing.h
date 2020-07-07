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
