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

import { ElementRef, EventEmitter } from "@angular/core";
import { Instant } from "@js-joda/core";
import { Observable, Subscription } from "rxjs";
import { WsMessage } from "./frame-message";
import { VideoUnitService } from "./video-unit.service";
import { SubscriptionSubject, VideoService } from "./video.service";

export class VideoUnitLibrary {
  private availability: [Instant, Instant][];

  constructor(private videoUnitService: VideoUnitService) {}

  getAvailability(): [Instant, Instant][] {
    return [];
  }

  getNext(unit: ?VideoUnit) {
    let i = this.videoUnits.findIndex((u) => {
      return u.id == unit.id;
    });
    if (i > -1 && i < this.videoUnits.length) {
      return this.videoUnits[i + 1];
    } else {
      return undefined;
    }
  }
}

export class VideoPlayer {
  private library: VideoUnitLibrary = new VideoUnitLibrary([]);
  private position: Instant;
  private subject?: SubscriptionSubject;
  private subscription?: Subscription;
  private videoObservable: Observable<WsMessage>;
  private positionEmitter: EventEmitter<Instant> = new EventEmitter<Instant>(
    true,
  );
  private frameEmitter: EventEmitter<FrameMessage> =
    new EventEmitter<FrameMessage>(true);

  constructor(
    private frameService: Observable<WsMessage>,
    private videoUnitService: VideoUnitService,
    private videoService: VideoService,
    private canvas: ElementRef<HTMLCanvasElement>,
  ) {
    this.position = Instant.MIN;
  }

  start(cameraId: number, time: Instant): EventEmitter<FrameMessage> {
    if (this.subscription) {
      this.pause();
    }
    position = time;
    // look up video unit

    this.subscription = this.frameService.subscribe((message: WsMessage) => {});
  }

  pause() {
    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
    }
  }

  resume() {
    this.start(this.position);
  }

  getPlayPosition(): EventEmitter<Instant> {}

  private processAvailability(observer) {
    this.videoUnitService
      .getVideoUnits(
        cameraId,
        beginTime.atZone(ZoneId.of("Z")),
        endTime.atZone(ZoneId.of("Z")),
      )
      .subscribe((units) => {
        this.videoUnitLibrary = new VideoUnitLibrary(units);
      });
  }

  getAvailability(
    cameraId: number,
    beginTime: Instant,
    endTime: Instant,
  ): Observable<[Instant, Instant][]> {}
}
