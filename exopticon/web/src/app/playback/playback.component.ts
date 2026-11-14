import { DatePipe } from "@angular/common";
import {
  ChangeDetectorRef,
  Component,
  ElementRef,
  OnDestroy,
  OnInit,
  ViewChild,
  ChangeDetectionStrategy,
} from "@angular/core";
import { ActivatedRoute } from "@angular/router";
import { Subscription } from "rxjs";
import { RecordingService } from "../recording.service";
import { DecodedFrame } from "./mkv-reader";
import { PlaybackController, PlaybackSpeed } from "./playback-controller";

interface TimelineRange {
  begin: number;
  end: number;
}

@Component({
  selector: "app-playback",
  imports: [DatePipe],
  templateUrl: "./playback.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrls: ["./playback.component.css"],
})
export class PlaybackComponent implements OnInit, OnDestroy {
  readonly scales = [
    { label: "24h", duration: 24 * 60 * 60 * 1000, tick: 3 * 60 * 60 * 1000 },
    { label: "1h", duration: 60 * 60 * 1000, tick: 10 * 60 * 1000 },
    { label: "30m", duration: 30 * 60 * 1000, tick: 5 * 60 * 1000 },
    { label: "10m", duration: 10 * 60 * 1000, tick: 2 * 60 * 1000 },
    { label: "1m", duration: 60 * 1000, tick: 10 * 1000 },
  ];
  scaleIndex = 2;
  get windowDuration(): number {
    return this.scales[this.scaleIndex].duration;
  }

  get ticks(): number[] {
    const interval = this.scales[this.scaleIndex].tick;
    // Align to local clock boundaries, including time zones with half-hour offsets.
    const midnight = new Date(this.windowStart);
    midnight.setHours(0, 0, 0, 0);
    const first =
      midnight.getTime() +
      Math.ceil((this.windowStart - midnight.getTime()) / interval) * interval;
    const ticks: number[] = [];
    for (let time = first; time < this.windowEnd; time += interval) {
      ticks.push(time);
    }
    return ticks;
  }

  get tickFormat(): string {
    return this.windowDuration <= 60000 ? "HH:mm:ss" : "HH:mm";
  }

  get currentTime(): number {
    return Date.now();
  }

  get shiftLabel(): string {
    const minutes = this.windowDuration / 120000;
    return minutes < 1
      ? "30 sec"
      : minutes >= 60
        ? `${minutes / 60} hr`
        : `${minutes} min`;
  }
  readonly speeds: PlaybackSpeed[] = [1, 2, 4];
  windowEnd = Date.now();
  windowStart = this.windowEnd - this.windowDuration;
  cameraName = "";
  ranges: TimelineRange[] = [];
  loading = false;
  error = false;
  readonly controller: PlaybackController;
  @ViewChild("canvas") canvas?: ElementRef<HTMLCanvasElement>;
  private routeSubscription?: Subscription;
  private rangeRequest?: Subscription;

  constructor(
    private route: ActivatedRoute,
    private recordings: RecordingService,
    private changeDetector: ChangeDetectorRef,
  ) {
    this.controller = new PlaybackController(
      recordings,
      undefined,
      (frame) => this.render(frame),
      () => this.changeDetector.markForCheck(),
    );
  }

  get position(): number {
    return this.controller.state.requestedTime ?? this.windowStart;
  }

  get displayedTime(): number | null {
    return this.controller.state.displayedTime;
  }

  ngOnInit(): void {
    this.routeSubscription = this.route.paramMap.subscribe((params) => {
      this.cameraName = params.get("camera_name") ?? "";
      this.controller.setCamera(this.cameraName);
      this.loadRanges();
    });
  }

  loadRanges(): void {
    this.rangeRequest?.unsubscribe();
    this.ranges = [];
    this.error = false;
    this.loading = !!this.cameraName;
    if (!this.cameraName) return;
    this.rangeRequest = this.recordings
      .getRanges(this.cameraName, this.windowStart, this.windowEnd)
      .subscribe({
        next: (response) => {
          this.ranges = response.ranges
            .map((range) => ({
              begin: Math.max(this.windowStart, Date.parse(range.beginTime)),
              end: Math.min(this.windowEnd, Date.parse(range.endTime)),
            }))
            .filter((range) => range.begin < range.end)
            .sort((a, b) => a.begin - b.begin);
          this.loading = false;
          this.changeDetector.markForCheck();
        },
        error: () => {
          this.error = true;
          this.loading = false;
          this.changeDetector.markForCheck();
        },
      });
  }

  shiftWindow(direction: number): void {
    const end = Math.min(
      Date.now(),
      this.windowEnd + (direction * this.windowDuration) / 2,
    );
    this.setWindow(end);
  }

  latest(): void {
    this.setWindow(Date.now());
  }

  setScale(index: number): void {
    if (index < 0 || index >= this.scales.length || index === this.scaleIndex)
      return;
    const anchor = this.playheadVisible
      ? this.position
      : (this.windowStart + this.windowEnd) / 2;
    this.scaleIndex = index;
    this.setWindow(Math.min(Date.now(), anchor + this.windowDuration / 2));
  }

  jumpToTime(value: string): void {
    const time = new Date(value).getTime();
    if (!Number.isFinite(time) || time > Date.now()) return;
    this.setWindow(Math.min(Date.now(), time + this.windowDuration / 2));
    this.seek(time);
  }

  private setWindow(end: number): void {
    this.windowEnd = end;
    this.windowStart = end - this.windowDuration;
    this.loadRanges();
  }

  percent(time: number): number {
    return ((time - this.windowStart) / this.windowDuration) * 100;
  }

  get playheadVisible(): boolean {
    return (
      this.controller.state.requestedTime !== null &&
      this.position >= this.windowStart &&
      this.position < this.windowEnd
    );
  }

  seek(time: number): void {
    const target = Math.max(
      this.windowStart,
      Math.min(time, this.windowEnd - 1),
    );
    void this.controller.seek(target);
  }

  seekPointer(event: PointerEvent): void {
    if (event.button !== 0) return;
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    if (bounds.width) {
      this.seek(
        this.windowStart +
          ((event.clientX - bounds.left) / bounds.width) * this.windowDuration,
      );
    }
  }

  seekButton(offset: number) {
    this.seek(this.position + offset);
  }

  seekKey(event: KeyboardEvent): void {
    const offsets: { [key: string]: number } = {
      ArrowLeft: -1000,
      ArrowRight: 1000,
      PageUp: 60000,
      PageDown: -60000,
    };
    if (event.key === "Home") {
      this.seek(this.windowStart);
    } else if (event.key === "End") {
      this.seek(this.windowEnd - 1);
    } else if (offsets[event.key] !== undefined) {
      this.seek(this.position + offsets[event.key]);
    } else {
      return;
    }
    event.preventDefault();
  }

  stepForward(): void {
    void this.controller.stepForward();
  }

  togglePlayback(): void {
    if (
      this.controller.state.phase === "playing" ||
      this.controller.state.phase === "buffering"
    )
      this.controller.pause();
    else this.controller.play();
  }

  setSpeed(speed: PlaybackSpeed): void {
    this.controller.setSpeed(speed);
  }

  jumpToNext(): void {
    void this.controller.jumpToNext();
  }

  retrySeek(): void {
    if (this.controller.canRetryTransition)
      void this.controller.retryTransition();
    else if (this.controller.state.requestedTime !== null)
      void this.controller.seek(this.controller.state.requestedTime);
  }

  exportPng() {
    let time = this.controller.state.displayedTime;
    if (!time || time == 0) return;
    let date = new Date(time);
    let cameraName = this.cameraName;
    let canvas = this.canvas?.nativeElement;
    if (!canvas) return;
    const img = canvas.toDataURL("image/png");
    const a = document.createElement("a");
    a.href = img;
    a.download = `${cameraName}-${date.toISOString()}.png`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }

  private render(frame: DecodedFrame | null): void {
    const canvas = this.canvas?.nativeElement;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Canvas rendering is unavailable");
    context.clearRect(0, 0, canvas.width, canvas.height);
    if (!frame) return;
    canvas.width = frame.sample.displayWidth;
    canvas.height = frame.sample.displayHeight;
    frame.sample.draw(context, 0, 0, canvas.width, canvas.height);
  }

  ngOnDestroy(): void {
    this.controller.destroy();
    this.routeSubscription?.unsubscribe();
    this.rangeRequest?.unsubscribe();
  }
}
