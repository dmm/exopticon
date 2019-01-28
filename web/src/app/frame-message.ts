
export class FrameResolution {
  type: string;
}

export class FrameMessage {
  cameraId: number;
  resolution: FrameResolution;
  jpeg: string;
}
