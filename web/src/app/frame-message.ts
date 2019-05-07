
export class AnalysisSource {
  analysis_engine_id: number;
  tag: string;
}

export class FrameSource {
  Camera: number;
  AnalysisEngine: AnalysisSource;
}

export class FrameMessage {
  cameraId: number;
  resolution: string;
  source: FrameSource;
  jpeg: string;
  video_unit_id: number;
  offset: number;
}
