
#ifndef __MPACK_FRAME_H
#define __MPACK_FRAME_H

struct FrameMessage {
        uint8_t *jpeg;
        int32_t jpeg_size;
        int64_t pts;
};

void send_frame_message(struct FrameMessage *msg);
void send_new_file_message(char *filename, char *iso_begin_time);
void send_end_file_message(char *filename, char *iso_end_time);
void send_log_message(char *message);

#endif // __MPACK_FRAME_H
