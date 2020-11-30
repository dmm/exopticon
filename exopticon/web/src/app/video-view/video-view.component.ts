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
  NgZone,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
} from "@angular/core";
import { Observable, Subscription } from "rxjs";
import { WsMessage } from "../frame-message";
import { Observation } from "../observation";

@Component({
  selector: "app-video-view",
  templateUrl: "./video-view.component.html",
  styleUrls: ["./video-view.component.css"],
})
export class VideoViewComponent implements OnInit {
  @Input() frameService?: Observable<WsMessage>;
  @Output() status = new EventEmitter<string>();
  @Output() frameOffset = new EventEmitter<number>();

  @ViewChild("obsCanvas", { static: true })
  canvas: ElementRef<HTMLCanvasElement>;

  private subscription?: Subscription;
  private img: HTMLImageElement;
  private isActive: boolean;
  private ctx: CanvasRenderingContext2D;

  constructor(
    private elementRef: ElementRef,
    private cdr: ChangeDetectorRef,
    private ngZone: NgZone
  ) {}

  ngOnInit() {}

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector("img");
    this.ctx = this.canvas.nativeElement.getContext("2d");
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty("frameService")) {
      if (changes["frameService"].currentValue) {
        this.activate();
      } else {
        this.deactivate();
      }
    }
  }

  ngOnDestroy() {
    this.deactivate();
  }

  activate() {
    this.isActive = false;
    this.status.emit("loading");

    if (this.subscription) {
      this.subscription.unsubscribe();
    }
    this.subscription = this.frameService.subscribe(
      (message: WsMessage) => {
        if (message.kind === "playbackEnd") {
          console.log("VideoView: playback End: " + message.id);
          this.status.emit("eof");
        } else if ((message.kind = "frame")) {
          if (!this.isActive) {
            this.isActive = true;
            this.status.emit("active");
          }
          if (this.img.complete) {
            this.img.onerror = () => {
              console.log("error!");
            };
            this.img.src = `data:image/jpeg;base64, ${message.jpeg}`;
            this.frameOffset.emit(message.offset);
            this.drawObservations(
              message.unscaledWidth,
              message.unscaledHeight,
              message.observations
            );
          }
        }
      },
      (error) => {
        console.log(`Caught websocket error! ${error}`);
      },
      () => {
        // complete
        console.log("VideoView: playback End subscription complete");
        this.status.emit("eof");
      }
    );
  }

  drawObservations(
    canvasWidth: number,
    canvasHeight: number,
    observations: Observation[]
  ) {
    //    this.ctx.clearRect(0, 0, this.canvas.nativeElement.width, this.canvas.nativeElement.height);
    this.canvas.nativeElement.width = canvasWidth;
    this.canvas.nativeElement.height = canvasHeight;
    this.ctx.strokeStyle = "#0F0";
    this.ctx.fillStyle = "#0F0";
    this.ctx.lineWidth = 5.0;
    this.ctx.font = "32pt sans";

    observations.forEach((o) => {
      if (o.tag == "motion") return;
      let width = o.lrX - o.ulX;
      let height = o.lrY - o.ulY;
      this.ctx.strokeRect(o.ulX, o.ulY, width, height);
      this.ctx.fillText(o.details, o.ulX, o.ulY);
      this.ctx.fillText(o.score.toString(), o.lrX, o.lrY + 40);
    });
  }

  deactivate() {
    this.status.emit("paused");
    this.isActive = false;

    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = undefined;
    }
  }
}
