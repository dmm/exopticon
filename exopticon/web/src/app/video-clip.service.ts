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

import { Injectable } from "@angular/core";
import { Observable } from "rxjs";
import { WsMessage } from "./frame-message";
import { VideoUnitService } from "./video-unit.service";
import { PlaybackSubject, VideoService } from "./video.service";

@Injectable({
  providedIn: "root",
})
export class VideoClipService {
  constructor(
    private videoUnitService: VideoUnitService,
    private videoService: VideoService
  ) {}

  playContext(videoUnitId: number, frameOffset: number): Observable<WsMessage> {
    let initialSubject: PlaybackSubject = {
      kind: "playback",
      id: Math.floor(Math.random() * Math.floor(10000)),
      videoUnitId: videoUnitId,
      offset: frameOffset,
    };

    this.videoService.connect();
    return this.videoService.getObservable(initialSubject);
  }
}
