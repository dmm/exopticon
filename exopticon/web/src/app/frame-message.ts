
import { Observation } from './observation';

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

export interface PlaybackSource {
  kind: 'playback';
  id: number;
}

export type FrameSource = AnalysisSource | CameraSource | PlaybackSource;

export class FrameMessage {
  source: FrameSource;
  resolution: CameraResolution;
  jpeg: string;
  videoUnitId: number;
  offset: number;
  unscaledWidth: number;
  unscaledHeight: number;
  observations: Observation[];
}
