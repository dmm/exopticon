
#ifndef __MPACK_FRAME_H
#define __MPACK_FRAME_H

struct FrameMessage {
        uint8_t *jpeg;
        int32_t jpeg_size;
        int32_t offset;
        int32_t unscaled_height;
        int32_t unscaled_width;
};

void send_frame_message(struct FrameMessage *msg);
void send_scaled_frame_message(struct FrameMessage *msg, const int32_t height);
void send_new_file_message(char *filename, char *iso_begin_time);
void send_end_file_message(char *filename, char *iso_end_time);
void send_log_message(int level, char *message);

#endif // __MPACK_FRAME_H
