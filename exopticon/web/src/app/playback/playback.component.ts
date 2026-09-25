import { DatePipe } from "@angular/common";
import {
  ChangeDetectorRef,
  Component,
  ElementRef,
  OnDestroy,
  OnInit,
  ViewChild,
} from "@angular/core";
import { ActivatedRoute, RouterLink } from "@angular/router";
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
  imports: [DatePipe, RouterLink],
  templateUrl: "./playback.component.html",
  styleUrls: ["./playback.component.css"],
})
export class PlaybackComponent implements OnInit, OnDestroy {
  readonly windowDuration = 60 * 60 * 1000;
  readonly ticks = [0, 15, 30, 45, 60];
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

  private setWindow(end: number): void {
    this.windowEnd = end;
    this.windowStart = end - this.windowDuration;
    this.loadRanges();
  }

  percent(time: number): number {
    return ((time - this.windowStart) / this.windowDuration) * 100;
  }

  get playheadVisible(): boolean {
    return this.position >= this.windowStart && this.position < this.windowEnd;
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
