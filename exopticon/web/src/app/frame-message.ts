/*
 * Exopticon - A free video surveillance system.
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

import { Observation } from "./observation";

export enum CameraResolution {
  Sd = "SD",
  Hd = "HD",
}

export interface CameraSource {
  kind: "camera";
  cameraId: number;
}

export interface AnalysisSource {
  kind: "analysisEngine";
  analysisEngineId: number;
  tag: string;
}

export interface PlaybackSource {
  kind: "playback";
  id: number;
}

export type FrameSource = AnalysisSource | CameraSource | PlaybackSource;

export class FrameMessage {
  kind: "frame";
  source: FrameSource;
  resolution: CameraResolution;
  jpeg: string;
  videoUnitId: number;
  offset: number;
  unscaledWidth: number;
  unscaledHeight: number;
  observations: Observation[];
}

export class PlaybackEnd {
  kind: "playbackEnd";
  id: number;
}

export type WsMessage = FrameMessage | PlaybackEnd;
