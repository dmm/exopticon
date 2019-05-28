
export enum CameraResolution {
  Sd = 'SD',
  Hd = 'HD',
}

export interface CameraSource {
  kind: 'camera';
  cameraId: number;
}

export interface AnalysisSource {
  kind: 'analysisEngine';
  analysisEngineId: number;
}

export type FrameSource = AnalysisSource | CameraSource;

export class FrameMessage {
  source: FrameSource;
  resolution: CameraResolution;
  jpeg: string;
  videoUnitId: number;
  offset: number;
}
