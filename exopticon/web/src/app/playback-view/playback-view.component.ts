import { Component, OnInit, ViewChild, ElementRef, SimpleChanges } from '@angular/core';
import { Router, ActivatedRoute, ParamMap } from '@angular/router';

import { Observable } from 'rxjs';
import { ChronoUnit, DateTimeFormatter, Duration, ZonedDateTime, ZoneId, ZoneOffset } from '@js-joda/core'
import '@js-joda/timezone'


import { WsMessage } from '../frame-message';
import { ObservationService } from '../observation.service';
import { Observation } from '../observation';
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
  public currentVideoService?: Observable<WsMessage>;

  private ctx: CanvasRenderingContext2D;
  private obCtx: CanvasRenderingContext2D;
  private units: [VideoUnit, any[], Observation[]][];
  private playbackProgress: number;
  private currentVideoUnit: [VideoUnit, any[], Observation[]] | null;
  private readonly playerDuration: Duration = Duration.ofHours(1);


  constructor(public route: ActivatedRoute,
              public videoService: VideoService,
              public observationService: ObservationService,
              public videoUnitService: VideoUnitService,
             ) {
    this.units = [];
    this.playbackProgress = -1;
  }

  progress(newOffset: number) {
    let currentPlaybackOffset = Duration.between(this.viewStartTime,
      this.currentVideoUnit[0].beginTime.plusNanos(newOffset * 1000)).toMillis();
    this.playbackProgress = currentPlaybackOffset / this.playerDuration.toMillis();
    this.drawProgressBar();
  }

  handleVideoStatus(newStatus: string) {
    if (newStatus === 'eof') {
      console.log('playback view: got eof');
      let unit = this.findNextVideoUnit(this.currentVideoUnit);
      console.log(' Next unit: ' + unit);
      if (unit) {
        this.currentVideoUnit = unit;
        this.play(unit[0], 0);
      }
    }
  }

  findNextVideoUnit(current: [VideoUnit, any[], Observation[]]): [VideoUnit, any[], Observation[]] | null {
    let next = null
    this.units.forEach((u, i, arr) => {
      if (u[0].id === current[0].id) {
        console.log("Found current!");
        if (arr.length - 1 > i) {
          next = arr[i + 1];
        }
      }
    });
    return next;
  }

  drawProgressBar() {
    // black out bar
    this.ctx.fillStyle = '#000';
    this.ctx.fillRect(0, 0, 1000, 20);

    this.units.forEach((u) => {
      let pos = Math.floor(this.calculateProgressPosition(u[0].beginTime));
      let end = Math.ceil(this.calculateProgressPosition(u[0].endTime));

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
    // clear out canvas
    this.obCtx.clearRect(0, 0, this.obCtx.canvas.width, this.obCtx.canvas.height);

    let canvasWidth = 1000;
    this.units.forEach((u) => {
      u[2].forEach(o => {
        let pos = this.calculateProgressPosition(u[0].beginTime, o.frameOffset)
        if (o.tag == 'motion') return;
        this.obCtx.fillStyle = '#F0F';
        this.obCtx.fillRect(pos, 0, 1, 20);
      });
    });
  }

  handleProgressClick(event: MouseEvent) {
    let progress = event.clientX / this.canvas.nativeElement.clientWidth;
    let offsetMillis = Math.floor(this.playerDuration.toMillis() * progress);
    let selectedTime = this.viewStartTime.plus(offsetMillis, ChronoUnit.MILLIS);
    let selectedUnit = this.findVideoUnitForTime(selectedTime);

    if (selectedUnit) {
      let fileOffsetMillis = Duration.between(selectedUnit[0].beginTime, selectedTime).toMillis() * 1000;
      this.currentVideoUnit = selectedUnit;
      this.play(selectedUnit[0], fileOffsetMillis);
    }
  }

  findVideoUnitForTime(time: ZonedDateTime): [VideoUnit, any[], Observation[]] | null {
    return this.units.find(([videoUnit, files, obs]) => {
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

    let offsetDuration = Duration.ofNanos(offset_micros * 1000);
    let displayMillis = Duration.between(this.viewStartTime, viewEndTime).toMillis();
    let displayOffsetMillis = Duration.between(this.viewStartTime, time).plusDuration(offsetDuration).toMillis();

    return (displayOffsetMillis / displayMillis) * 1000;
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
      this.units = units;
      this.drawProgressBar();
      this.drawObservations();
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
      this.units = units;
      this.drawProgressBar();
      this.drawObservations();
    });
  }
}
