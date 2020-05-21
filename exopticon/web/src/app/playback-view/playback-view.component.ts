import { Component, OnInit, ViewChild, ElementRef, SimpleChanges } from '@angular/core';
import { Router, ActivatedRoute, ParamMap } from '@angular/router';

import { Observable } from 'rxjs';
import { ChronoUnit, DateTimeFormatter, Duration, ZonedDateTime, ZoneId, ZoneOffset } from '@js-joda/core'
import '@js-joda/timezone'


import { FrameMessage } from '../frame-message';
import { ObservationService } from '../observation.service';
import { VideoUnit } from '../video-unit';
import { VideoUnitService } from '../video-unit.service';
import { VideoService, SubscriptionSubject } from '../video.service';

@Component({
  selector: 'app-playback-view',
  templateUrl: './playback-view.component.html',
  styleUrls: ['./playback-view.component.css']
})
export class PlaybackViewComponent implements OnInit {
  @ViewChild('canvas', { static: true })
  canvas: ElementRef<HTMLCanvasElement>;

  @ViewChild('observations', { static: true })
  obCanvas: ElementRef<HTMLCanvasElement>;

  public enabled: boolean;
  public viewStartFormatted: string;
  public viewStartTime: ZonedDateTime;
  public currentTime: ZonedDateTime;
  public playbackEnabled: boolean = false;
  public playbackSubject: SubscriptionSubject | null;
  public cameraId: number;
  public currentVideoService?: Observable<FrameMessage>;

  private ctx: CanvasRenderingContext2D;
  private obCtx: CanvasRenderingContext2D;
  private units: VideoUnit[];
  private observations: any[];
  private playbackProgress: number;
  private currentVideoUnit: VideoUnit | null;
  private readonly playerDuration: Duration = Duration.ofHours(2);


  constructor(public route: ActivatedRoute,
    public videoService: VideoService,
    public observationService: ObservationService,
    public videoUnitService: VideoUnitService) {
    this.units = [];
    this.playbackProgress = -1;
  }

  progress(newOffset: number) {
    let currentPlaybackOffset = Duration.between(this.viewStartTime,
      this.currentVideoUnit.beginTime.plusNanos(newOffset * 1000)).toMillis();
    this.playbackProgress = currentPlaybackOffset / this.playerDuration.toMillis();
    this.drawProgressBar();
  }

  drawProgressBar() {
    // black out bar
    this.ctx.fillStyle = '#000';
    this.ctx.fillRect(0, 0, 1000, 20);

    this.units.forEach((u) => {
      let pos = Math.floor(this.calculateProgressPosition(u.beginTime));
      let end = Math.ceil(this.calculateProgressPosition(u.endTime));

      if (pos == -1 && end == -1) return;
      if (pos == -1) pos = 0;
      if (end == -1) end = 1000;


      let size = end - pos;
      this.ctx.fillStyle = '#0F0';
      this.ctx.fillRect(pos, 0, size, 20);
    });
    if (this.playbackProgress > 0) {
      this.ctx.fillStyle = '#F00';
      this.ctx.fillRect(1000 * this.playbackProgress - 2, 0, 4, 20);
    }
  }

  drawObservations() {
    return;
    let canvasWidth = 1000;
    this.observations.forEach((o) => {
      let pos = this.calculateProgressPosition(o[1].beginTime, o[0].frameOffset)
      console.log('calculated pos: ' + pos);
      let x = canvasWidth * pos;
      this.obCtx.fillStyle = '#F00';
      this.obCtx.fillRect(x, 0, 1, 20);
    });
  }

  handleProgressClick(event: MouseEvent) {
    let progress = event.clientX / this.canvas.nativeElement.clientWidth;
    let offsetMillis = Math.floor(this.playerDuration.toMillis() * progress);
    let selectedTime = this.viewStartTime.plus(offsetMillis, ChronoUnit.MILLIS);
    let selectedUnit = this.findVideoUnitForTime(selectedTime);

    if (selectedUnit) {
      let fileOffsetMillis = Duration.between(selectedUnit.beginTime, selectedTime).toMillis() * 1000;
      this.currentVideoUnit = selectedUnit;
      this.play(selectedUnit, fileOffsetMillis);
    }
  }

  findVideoUnitForTime(time: ZonedDateTime): VideoUnit | null {
    return this.units.find((videoUnit) => {
      return videoUnit.beginTime.isBefore(time) && videoUnit.endTime.isAfter(time);
    });
  }

  play(videoUnit: VideoUnit, offset: number) {
    if (this.playbackEnabled) {
      this.stop();
    }
    this.playbackSubject = {
      kind: 'playback',
      id: this.getRandomInt(1000),
      videoUnitId: videoUnit.id,
      offset: offset,
    };
    this.currentVideoService = this.videoService.getObservable(this.playbackSubject);
    this.playbackEnabled = true;
  }

  stop() {
    this.playbackEnabled = false;
    this.playbackSubject = null;
    this.currentVideoService = null;
    this.playbackProgress = -1;
  }

  getRandomInt(max: number) {
    return Math.floor(Math.random() * Math.floor(max));
  }

  calculateProgressPosition(time: ZonedDateTime, offset_micros: number = 0): number {
    let viewEndTime = this.viewStartTime.plusMinutes(this.playerDuration.toMinutes());
    if (time.isBefore(this.viewStartTime) || time.isAfter(viewEndTime)) {
      return -1;
    }

    let displayDuration = Duration.between(this.viewStartTime, viewEndTime).toMillis();
    let offsetDuration = Duration.between(this.viewStartTime, time).toMillis() + (offset_micros / 1000);

    return (offsetDuration / displayDuration) * 1000;
  }

  setViewStartTime(time: ZonedDateTime) {
    this.viewStartTime = time;
    const formatter = DateTimeFormatter.ofPattern('yyyy-MM-dd HH:mm');
    this.viewStartFormatted = this.viewStartTime.format(formatter);
  }

  timeShift(shiftAmount: number) {
    if (shiftAmount < 0) {
      this.setViewStartTime(this.viewStartTime.plusMinutes(-60));
    } else {
      this.setViewStartTime(this.viewStartTime.plusMinutes(60));
    }

    let viewEndTime = this.viewStartTime.plusMinutes(this.playerDuration.toMinutes());
    this.videoUnitService.getVideoUnits(this.cameraId, this.viewStartTime, viewEndTime
    ).subscribe((units) => {
      this.units = units.map((u) => {
        u.beginTime = ZonedDateTime.parse(u.beginTime + 'Z');
        u.endTime = ZonedDateTime.parse(u.endTime + 'Z');
        return u;
      });
      this.drawProgressBar();
    });

  }

  ngOnInit() {
    this.cameraId = parseInt(this.route.snapshot.paramMap.get('id'), 10);
    this.videoService.connect();
    this.ctx = this.canvas.nativeElement.getContext('2d');
    this.obCtx = this.obCanvas.nativeElement.getContext('2d');
    this.currentTime = ZonedDateTime.now(ZoneOffset.UTC);
    this.setViewStartTime(
      this.currentTime.minusMinutes(this.playerDuration.toMinutes() / 2));
    let viewEndTime = this.viewStartTime.plusMinutes(this.playerDuration.toMinutes());
    this.enabled = true;
    this.drawProgressBar();
    this.videoUnitService.getVideoUnits(this.cameraId, this.viewStartTime, viewEndTime
    ).subscribe((units) => {
      this.units = units.map((u) => {
        u.beginTime = ZonedDateTime.parse(u.beginTime + 'Z');
        u.endTime = ZonedDateTime.parse(u.endTime + 'Z');
        return u;
      });
      this.drawProgressBar();
    });
    return;
    this.observationService.getObservations(this.cameraId, this.viewStartTime, viewEndTime).subscribe((obs) => {
      this.observations = obs;
      this.drawObservations();
    });
  }
}
