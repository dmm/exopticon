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

export type CameraName = string;
export type CameraId = CameraName;

export interface ResourceMetadata {
  name: string;
  displayName: string;
}

export type StreamSource =
  | { type: "staticRtsp"; url: string }
  | { type: "onvif"; deviceUrl: string; profileToken: string };

export interface PtzSettings {
  deviceUrl: string | null;
  profileToken: string | null;
  xStepSize: number;
  yStepSize: number;
}

export interface CameraSpec {
  storageGroupName: string;
  streamSource: StreamSource;
  ptz: PtzSettings | null;
  enabled: boolean;
}

export interface CameraStatus {
  phase: "disabled" | "starting" | "running" | "stopped" | "error";
  lastStartedAt?: string;
  videoCodec?: string;
  audioCodec?: string;
  averageBitrate?: number;
  errorMessage?: string;
}

export interface Camera {
  metadata: ResourceMetadata;
  spec: CameraSpec;
  status: CameraStatus;
}
