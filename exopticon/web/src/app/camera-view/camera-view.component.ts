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

import {
  ChangeDetectorRef,
  Component,
  ElementRef,
  EventEmitter,
  Input,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
} from "@angular/core";
import { Observable, Subscription } from "rxjs";
import { Camera } from "../camera";
import { CameraResolution, WsMessage } from "../frame-message";
import { SubscriptionSubject, VideoService } from "../video.service";
import { WebrtcService } from "../webrtc.service";

enum CameraViewStatus {
  New,
  Connecting,
  Playing,
}

@Component({
  selector: "app-camera-view",
  templateUrl: "./camera-view.component.html",
  styleUrls: ["./camera-view.component.css"],
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() enabled: boolean;
  @Input() videoService: VideoService;
  @Input() resolution: CameraResolution;

  @Output() isVisible = new EventEmitter<boolean>();

  @ViewChild("wrapperDiv") wrapperDiv: ElementRef;

  @ViewChild("videoElement") videoElement: ElementRef;

  public status: string;

  private videoSubject: SubscriptionSubject;
  public frameService?: Observable<WsMessage>;
  private mediaStream?: MediaStream = null;
  private state: CameraViewStatus = CameraViewStatus.New;
  private subscription: Subscription = null;

  constructor(
    private changeRef: ChangeDetectorRef,
    private webrtcService: WebrtcService,
  ) {}

  ngOnInit() {
    if (this.enabled) {
      this.activate();
    }
  }

  ngAfterViewInit() {
    if (this.state === CameraViewStatus.New) {
      this.onVideoStatusChange("loading...");
      this.activate();
      this.setMediaSource();
      this.state = CameraViewStatus.Connecting;
    }
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty("enabled")) {
      if (changes["enabled"].currentValue) {
        this.activate();
      } else {
        this.deactivate();
      }
    }

    if (changes.hasOwnProperty("resolution")) {
      // handle changing resolution
    }
  }

  setMediaSource() {
    if (this.mediaStream !== null) {
      let video = this.videoElement.nativeElement as HTMLVideoElement;
      video.srcObject = this.mediaStream;
      video.muted = true;
      video.autoplay = true;
      video.onloadeddata = this.genStatusHandler("active");
      video.onpause = this.genStatusHandler("loading");

      video.onemptied = () => {
        console.log("&&&&&&CAMERA " + this.camera.id + " EMPTIED!");
      };
      video.onwaiting = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " WAITING!");
      };
      video.onstalled = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " STALLED!");
      };
      video.onended = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " ENDED!");
        this.state = CameraViewStatus.New;
      };

      video.onplaying = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " PLAAAAAYING!");
        this.state = CameraViewStatus.Playing;
      };
      video.onstalled = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " STALLED!");
      };
      video.onwaiting = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " WAITING!");
      };
      video.oncanplay = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " CANPLAY!");
      };
      video.onloadeddata = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " LOADEDDATA!");
      };
      video.ontimeupdate = () => {
        console.log("&&&&&&&CAMERA " + this.camera.id + " TIMEUPDATE!");
      };
    } else {
      let video = this.videoElement.nativeElement as HTMLVideoElement;
      video.pause();
      video.srcObject = null;
    }
  }

  activate() {
    this.videoSubject = {
      kind: "camera",
      cameraId: this.camera.id,
      resolution: this.resolution,
    };
    //    this.frameService = this.videoService.getObservable(this.videoSubject);
    this.subscription = this.webrtcService.subscribe(this.camera.id).subscribe(
      (m) => {
        this.mediaStream = m;
        this.setMediaSource();
      },
      (err) => {
        this.mediaStream = null;
        this.setMediaSource();
      },
    );
  }

  deactivate() {
    if (this.subscription !== null) {
      this.subscription.unsubscribe();
    }
    if (this.videoElement !== undefined) {
      let video = this.videoElement.nativeElement as HTMLVideoElement;
      video.pause();
    }
  }

  setStatus(event) {
    this.status = "active";
  }

  genStatusHandler(status: string) {
    return () => {
      this.onVideoStatusChange(status);
    };
  }

  onVideoStatusChange(status: string) {
    setTimeout(() => {
      this.status = status;
    }, 0);
  }
}
