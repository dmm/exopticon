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

import { Component, ElementRef, OnInit, ViewChild } from "@angular/core";
import { ActivatedRoute } from "@angular/router";
import { Observable, Subscription } from "rxjs";
import { ElementVisibleService } from "../element-visible.service";
import { WsMessage } from "../frame-message";
import { Observation } from "../observation";
import { ObservationService } from "../observation.service";
import { VideoViewComponent } from "../video-view/video-view.component";
import { PlaybackSubject, VideoService } from "../video.service";

@Component({
  selector: "app-alert-view",
  templateUrl: "./alert-view.component.html",
  styleUrls: ["./alert-view.component.css"],
})
export class AlertViewComponent implements OnInit {
  public observationId: number;
  public currentVideoService: Observable<WsMessage>;
  private observation: Observation;
  private subscription?: Subscription;

  @ViewChild("videoDiv", { static: true })
  videoDiv: ElementRef<VideoViewComponent>;

  constructor(
    public route: ActivatedRoute,
    private observationService: ObservationService,
    private videoService: VideoService,
    private visibilityService: ElementVisibleService,
  ) {}

  stop(): void {
    if (this.subscription) {
      this.subscription.unsubscribe();
    }
    this.currentVideoService = null;
  }

  playClip(): void {
    let initialSubject: PlaybackSubject = {
      kind: "playback",
      id: Math.floor(Math.random() * Math.floor(10000)),
      videoUnitId: this.observation.videoUnitId,
      offset: this.observation.frameOffset,
    };
    let stopOffset = this.observation.frameOffset + 1000000 * 4;
    this.videoService.connect();
    if (this.subscription) {
      this.subscription.unsubscribe();
    }

    this.currentVideoService = this.videoService.getObservable(initialSubject);
    this.subscription = this.currentVideoService.subscribe((frame) => {
      if (frame.kind == "frame" && frame.offset >= stopOffset) {
        this.playClip();
      }
    });
  }

  handleVideoStatus(newStatus: string) {
    console.log("status: " + newStatus);
    if (newStatus === "eof") {
      console.log("got end of file!");
      this.playClip();
    }
  }

  ngOnInit(): void {
    //    const isVisible = this.visibilityService.elementVisible(this.videoDiv.elementRef);
    this.observationId = parseInt(this.route.snapshot.paramMap.get("id"), 10);

    this.observationService
      .getObservation(this.observationId)
      .subscribe((observation) => {
        console.log("Got observation! " + observation.toString());
        this.observation = observation;
        /*
      isVisible.subscribe((visible) => {
        if (visible) {
          this.playClip();
        } else {
          this.stop()
        }
      });
*/
      });
  }
}
