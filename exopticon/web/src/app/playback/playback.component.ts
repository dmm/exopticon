import { DatePipe } from "@angular/common";
import { ChangeDetectorRef, Component, OnDestroy, OnInit } from "@angular/core";
import { ActivatedRoute, RouterLink } from "@angular/router";
import { Subscription } from "rxjs";
import { RecordingService } from "../recording.service";

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
  readonly frameDuration = 1000 / 30;
  readonly ticks = [0, 15, 30, 45, 60];
  windowEnd = Date.now();
  windowStart = this.windowEnd - this.windowDuration;
  position = this.windowStart;
  cameraName = "";
  ranges: TimelineRange[] = [];
  loading = false;
  error = false;
  playing = false;
  private routeSubscription?: Subscription;
  private rangeRequest?: Subscription;
  private timer?: ReturnType<typeof setInterval>;

  constructor(
    private route: ActivatedRoute,
    private recordings: RecordingService,
    private changeDetector: ChangeDetectorRef,
  ) {}

  ngOnInit(): void {
    this.routeSubscription = this.route.paramMap.subscribe((params) => {
      this.cameraName = params.get("camera_name") ?? "";
      this.loadRanges();
    });
  }

  loadRanges(): void {
    this.pause();
    this.rangeRequest?.unsubscribe();
    this.ranges = [];
    this.error = false;
    this.loading = !!this.cameraName;
    if (!this.cameraName) {
      return;
    }
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
    this.position = Math.max(
      this.windowStart,
      Math.min(this.position, this.windowEnd - 1),
    );
    this.loadRanges();
  }

  percent(time: number): number {
    return ((time - this.windowStart) / this.windowDuration) * 100;
  }

  get available(): boolean {
    return this.ranges.some(
      (range) => range.begin <= this.position && this.position < range.end,
    );
  }

  private nextAvailable(time: number): number | undefined {
    const range = this.ranges.find((candidate) => candidate.end > time);
    return range ? Math.max(time, range.begin) : undefined;
  }

  get canAdvance(): boolean {
    return (
      !this.loading &&
      !this.error &&
      this.nextAvailable(this.position + this.frameDuration) !== undefined
    );
  }

  seek(time: number): void {
    this.position = Math.max(
      this.windowStart,
      Math.min(time, this.windowEnd - 1),
    );
  }

  seekPointer(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
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

  togglePlay(): void {
    if (this.playing) {
      this.pause();
      return;
    }
    const next = this.nextAvailable(this.position);
    if (next === undefined || !this.canAdvance) {
      return;
    }
    this.position = next;
    this.playing = true;
    let previous = performance.now();
    this.timer = setInterval(() => {
      const now = performance.now();
      this.advance(now - previous);
      previous = now;
      this.changeDetector.markForCheck();
    }, 100);
  }

  private advance(milliseconds: number): void {
    const next = this.nextAvailable(this.position + milliseconds);
    if (next === undefined) {
      this.pause();
      return;
    }
    this.position = next;
  }

  stepForward(): void {
    if (!this.playing && this.canAdvance) {
      this.advance(this.frameDuration);
    }
  }

  pause(): void {
    this.playing = false;
    if (this.timer !== undefined) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
  }

  ngOnDestroy(): void {
    this.pause();
    this.routeSubscription?.unsubscribe();
    this.rangeRequest?.unsubscribe();
  }
}
