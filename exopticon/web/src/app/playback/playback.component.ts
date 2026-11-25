import { DatePipe } from "@angular/common";
import {
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  Component,
  ElementRef,
  OnDestroy,
  OnInit,
  ViewChild,
} from "@angular/core";
import { ActivatedRoute } from "@angular/router";
import { Subscription } from "rxjs";
import { RecordingService } from "../recording.service";
import {
  ClipExportCancelledError,
  ClipExportOptions,
  ClipExportProgress,
  ClipExportResult,
  exportClip,
  MAX_CLIP_DURATION_MS,
} from "./clip-exporter";
import {
  ceilMilliseconds,
  DecodedFrame,
  parseRecordingTimeNanoseconds,
} from "./mkv-reader";
import { PlaybackController, PlaybackSpeed } from "./playback-controller";

interface TimelineRange {
  begin: number;
  end: number;
}

type ExportHandle = "start" | "end";

interface ExportHandleDrag {
  handle: ExportHandle;
  pointerId: number;
  element: HTMLElement;
  timeline: HTMLElement;
}

const DEFAULT_CLIP_DURATION_MS = 30 * 1000;
const MIN_CLIP_HANDLE_GAP_MS = 1000;
const BLOB_REVOKE_DELAY_MS = 60 * 1000;

@Component({
  selector: "app-playback",
  imports: [DatePipe],
  templateUrl: "./playback.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrls: ["./playback.component.css"],
  host: {
    "(document:visibilitychange)": "onVisibilityChange()",
  },
})
export class PlaybackComponent implements OnInit, OnDestroy {
  readonly scales = [
    { label: "24h", duration: 24 * 60 * 60 * 1000, tick: 3 * 60 * 60 * 1000 },
    { label: "1h", duration: 60 * 60 * 1000, tick: 10 * 60 * 1000 },
    { label: "30m", duration: 30 * 60 * 1000, tick: 5 * 60 * 1000 },
    { label: "10m", duration: 10 * 60 * 1000, tick: 2 * 60 * 1000 },
    { label: "1m", duration: 60 * 1000, tick: 10 * 1000 },
  ];
  readonly maxClipDurationMs = MAX_CLIP_DURATION_MS;
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
  exportMode = false;
  exportStart: number | null = null;
  exportEnd: number | null = null;
  exportRunning = false;
  exportProgress: ClipExportProgress | null = null;
  exportError: string | null = null;
  exportNotice: string | null = null;
  readonly controller: PlaybackController;
  runClipExport: (options: ClipExportOptions) => Promise<ClipExportResult> =
    exportClip;
  @ViewChild("canvas") canvas?: ElementRef<HTMLCanvasElement>;
  @ViewChild("timeline") timeline?: ElementRef<HTMLElement>;
  private routeSubscription?: Subscription;
  private rangeRequest?: Subscription;
  private dragPointer?: number;
  private dragElement?: HTMLElement;
  private exportHandleDrag?: ExportHandleDrag;
  private previewTimer?: ReturnType<typeof setTimeout>;
  private pendingPreviewTarget?: number;
  private lastPreviewAt = -Infinity;
  private readonly previewInterval = 150;
  private exportAbort?: AbortController;

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
      this.endDrag();
      this.endExportHandleDrag();
      this.resetExportMode();
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
              // Round begin upward so millisecond selections at the left edge
              // still satisfy the backend's microsecond availability check.
              begin: Math.max(
                this.windowStart,
                ceilMilliseconds(
                  parseRecordingTimeNanoseconds(range.beginTime),
                ),
              ),
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

  get exportSelectionVisible(): boolean {
    return (
      this.exportMode &&
      this.exportStart !== null &&
      this.exportEnd !== null &&
      this.exportEnd > this.exportStart &&
      this.exportEnd > this.windowStart &&
      this.exportStart < this.windowEnd
    );
  }

  get exportSelectionLeftPercent(): number {
    if (this.exportStart === null) return 0;
    return this.percent(Math.max(this.windowStart, this.exportStart));
  }

  get exportSelectionWidthPercent(): number {
    if (this.exportStart === null || this.exportEnd === null) return 0;
    const left = Math.max(this.windowStart, this.exportStart);
    const right = Math.min(this.windowEnd, this.exportEnd);
    return Math.max(0, this.percent(right) - this.percent(left));
  }

  get exportStartVisible(): boolean {
    return (
      this.exportMode &&
      this.exportStart !== null &&
      this.exportStart >= this.windowStart &&
      this.exportStart <= this.windowEnd
    );
  }

  get exportEndVisible(): boolean {
    return (
      this.exportMode &&
      this.exportEnd !== null &&
      this.exportEnd >= this.windowStart &&
      this.exportEnd <= this.windowEnd
    );
  }

  get exportDurationLabel(): string {
    if (
      this.exportStart === null ||
      this.exportEnd === null ||
      this.exportEnd <= this.exportStart
    ) {
      return "-- / 5:00 max";
    }
    const totalSeconds = Math.round((this.exportEnd - this.exportStart) / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")} / 5:00 max`;
  }

  get exportValidationError(): string | null {
    if (!this.exportMode) return null;
    if (this.exportStart === null || this.exportEnd === null) {
      return "Select a start and end time.";
    }
    if (this.exportEnd <= this.exportStart) {
      return "End time must be after start time.";
    }
    if (this.exportEnd - this.exportStart > MAX_CLIP_DURATION_MS) {
      return "Clip duration cannot exceed 5 minutes.";
    }
    if (!this.loading && !this.error) {
      const visibleStart = Math.max(this.exportStart, this.windowStart);
      const visibleEnd = Math.min(this.exportEnd, this.windowEnd);
      if (visibleStart < visibleEnd) {
        const covered = this.ranges.some(
          (range) => range.begin <= visibleStart && range.end >= visibleEnd,
        );
        if (!covered) {
          return "Selected range includes unrecorded time or a gap. Adjust start and end to a continuous recording.";
        }
      }
    }
    return null;
  }

  get exportProgressText(): string {
    if (!this.exportProgress) return "";
    const { stage, completedFragments, totalFragments } = this.exportProgress;
    switch (stage) {
      case "resolving":
        return "Checking recording continuity…";
      case "downloading":
        return `Downloading fragment ${Math.min(completedFragments + 1, totalFragments)} of ${totalFragments}…`;
      case "muxing":
        return `Processing fragment ${Math.min(completedFragments + 1, totalFragments)} of ${totalFragments}…`;
      case "finalizing":
        return "Finalizing MP4…";
    }
  }

  get exportStartInputValue(): string {
    return this.toLocalDateTimeInput(this.exportStart);
  }

  get exportEndInputValue(): string {
    return this.toLocalDateTimeInput(this.exportEnd);
  }

  toggleExportMode(): void {
    if (this.exportMode) {
      this.resetExportMode();
    } else {
      this.exportMode = true;
      this.exportError = null;
      this.exportNotice = null;
      this.initializeExportSelection();
    }
    this.changeDetector.markForCheck();
  }

  private resetExportMode(): void {
    this.cancelExport();
    this.endExportHandleDrag();
    this.exportMode = false;
    this.exportStart = null;
    this.exportEnd = null;
    this.exportError = null;
    this.exportNotice = null;
    this.exportProgress = null;
  }

  private initializeExportSelection(): void {
    const requested = this.controller.state.requestedTime;
    const activeRange =
      requested !== null
        ? this.ranges.find(
            (range) => requested >= range.begin && requested <= range.end,
          )
        : undefined;

    if (activeRange && requested !== null) {
      let start = Math.max(
        activeRange.begin,
        Math.floor(requested / 1000) * 1000,
      );
      let end = Math.min(activeRange.end, start + DEFAULT_CLIP_DURATION_MS);
      if (end <= start) {
        end = activeRange.end;
        start = Math.max(activeRange.begin, end - DEFAULT_CLIP_DURATION_MS);
      }
      this.exportStart = Math.trunc(start);
      this.exportEnd = Math.trunc(end);
      return;
    }

    if (this.ranges.length > 0) {
      const latestRange = this.ranges[this.ranges.length - 1];
      const end = latestRange.end;
      const start = Math.max(latestRange.begin, end - DEFAULT_CLIP_DURATION_MS);
      this.exportStart = Math.trunc(start);
      this.exportEnd = Math.trunc(end);
      return;
    }

    const anchor = this.playheadVisible
      ? this.position
      : (this.windowStart + this.windowEnd) / 2;
    const half = DEFAULT_CLIP_DURATION_MS / 2;
    const start = Math.max(
      this.windowStart,
      Math.floor((anchor - half) / 1000) * 1000,
    );
    const end = Math.min(this.windowEnd, start + DEFAULT_CLIP_DURATION_MS);
    this.exportStart = Math.trunc(start);
    this.exportEnd = Math.trunc(
      end > start ? end : Math.min(this.windowEnd, start + 1000),
    );
  }

  setExportStartFromPlayhead(): void {
    if (!this.exportMode || this.exportRunning) return;
    const start = Math.trunc(this.position);
    this.exportStart = start;
    if (this.exportEnd === null || this.exportEnd <= start) {
      const enclosing = this.ranges.find(
        (range) => start >= range.begin && start <= range.end,
      );
      const maxEnd = enclosing
        ? Math.min(enclosing.end, start + DEFAULT_CLIP_DURATION_MS)
        : Math.min(Date.now(), start + DEFAULT_CLIP_DURATION_MS);
      this.exportEnd = Math.trunc(
        maxEnd > start ? maxEnd : start + MIN_CLIP_HANDLE_GAP_MS,
      );
    } else if (this.exportEnd - start > MAX_CLIP_DURATION_MS) {
      this.exportEnd = start + MAX_CLIP_DURATION_MS;
    }
    this.exportError = null;
    this.exportNotice = null;
    this.changeDetector.markForCheck();
  }

  setExportEndFromPlayhead(): void {
    if (!this.exportMode || this.exportRunning) return;
    const end = Math.trunc(this.position);
    this.exportEnd = end;
    if (this.exportStart === null || this.exportStart >= end) {
      const enclosing = this.ranges.find(
        (range) => end >= range.begin && end <= range.end,
      );
      const minStart = enclosing
        ? Math.max(enclosing.begin, end - DEFAULT_CLIP_DURATION_MS)
        : end - DEFAULT_CLIP_DURATION_MS;
      this.exportStart = Math.trunc(
        minStart < end ? minStart : end - MIN_CLIP_HANDLE_GAP_MS,
      );
    } else if (end - this.exportStart > MAX_CLIP_DURATION_MS) {
      this.exportStart = end - MAX_CLIP_DURATION_MS;
    }
    this.exportError = null;
    this.exportNotice = null;
    this.changeDetector.markForCheck();
  }

  setExportStartInput(value: string): void {
    if (this.exportRunning) return;
    const time = new Date(value).getTime();
    if (!Number.isFinite(time)) return;
    this.exportStart = Math.trunc(time);
    this.exportError = null;
    this.exportNotice = null;
    this.changeDetector.markForCheck();
  }

  setExportEndInput(value: string): void {
    if (this.exportRunning) return;
    const time = new Date(value).getTime();
    if (!Number.isFinite(time)) return;
    this.exportEnd = Math.trunc(time);
    this.exportError = null;
    this.exportNotice = null;
    this.changeDetector.markForCheck();
  }

  startExportHandlePointer(handle: ExportHandle, event: PointerEvent): void {
    if (
      event.button !== 0 ||
      this.exportRunning ||
      this.exportHandleDrag !== undefined
    )
      return;
    event.stopPropagation?.();
    event.preventDefault?.();
    const element = event.currentTarget as HTMLElement;
    const timeline =
      this.timeline?.nativeElement ??
      (element.parentElement as HTMLElement | null);
    if (!timeline) return;
    element.focus({ preventScroll: true });
    this.endDrag();
    this.exportHandleDrag = {
      handle,
      pointerId: event.pointerId,
      element,
      timeline,
    };
    element.setPointerCapture?.(event.pointerId);
    this.lastPreviewAt = -Infinity;
    this.updateExportHandleDrag(event);
  }

  moveExportHandlePointer(event: PointerEvent): void {
    if (
      this.exportHandleDrag !== undefined &&
      event.pointerId === this.exportHandleDrag.pointerId
    ) {
      event.stopPropagation?.();
      this.updateExportHandleDrag(event);
    }
  }

  releaseExportHandlePointer(event: PointerEvent): void {
    const drag = this.exportHandleDrag;
    if (!drag || event.pointerId !== drag.pointerId) return;
    event.stopPropagation?.();
    if (event.type === "pointerup") {
      this.updateExportHandleDrag(event, false);
    }
    const target = drag.handle === "start" ? this.exportStart : this.exportEnd;
    this.endExportHandleDrag();
    if (target !== null) {
      void this.controller.seek(this.clampTime(target));
    }
  }

  exportHandleKey(handle: ExportHandle, event: KeyboardEvent): void {
    if (this.exportRunning) return;
    const step = event.shiftKey ? 10000 : 1000;
    let delta = 0;
    if (event.key === "ArrowLeft") delta = -step;
    else if (event.key === "ArrowRight") delta = step;
    else return;

    event.stopPropagation?.();
    event.preventDefault();
    const current = handle === "start" ? this.exportStart : this.exportEnd;
    if (current === null) return;
    const updated = this.applyHandleTime(handle, current + delta);
    if (updated !== null) {
      void this.controller.seek(this.clampTime(updated));
    }
  }

  private updateExportHandleDrag(event: PointerEvent, preview = true): void {
    const drag = this.exportHandleDrag;
    if (!drag) return;
    const bounds = drag.timeline.getBoundingClientRect();
    if (!bounds.width) return;
    const raw =
      this.windowStart +
      ((event.clientX - bounds.left) / bounds.width) * this.windowDuration;
    const clamped = Math.max(this.windowStart, Math.min(raw, this.windowEnd));
    const updated = this.applyHandleTime(drag.handle, clamped);
    if (updated === null) return;
    const previewTime = this.clampTime(updated);
    this.controller.selectTime(previewTime);
    if (preview) {
      this.schedulePreview(previewTime);
    }
  }

  private applyHandleTime(
    handle: ExportHandle,
    rawTime: number,
  ): number | null {
    const time = Math.trunc(rawTime);
    if (handle === "start") {
      if (this.exportEnd === null) return null;
      const minStart = this.exportEnd - MAX_CLIP_DURATION_MS;
      const maxStart = this.exportEnd - MIN_CLIP_HANDLE_GAP_MS;
      if (maxStart < minStart) return null;
      this.exportStart = Math.max(minStart, Math.min(time, maxStart));
      this.exportError = null;
      this.exportNotice = null;
      this.changeDetector.markForCheck();
      return this.exportStart;
    } else {
      if (this.exportStart === null) return null;
      const minEnd = this.exportStart + MIN_CLIP_HANDLE_GAP_MS;
      const maxEnd = this.exportStart + MAX_CLIP_DURATION_MS;
      this.exportEnd = Math.max(minEnd, Math.min(time, maxEnd));
      this.exportError = null;
      this.exportNotice = null;
      this.changeDetector.markForCheck();
      return this.exportEnd;
    }
  }

  private endExportHandleDrag(): void {
    if (this.previewTimer !== undefined) clearTimeout(this.previewTimer);
    this.previewTimer = undefined;
    this.pendingPreviewTarget = undefined;
    const drag = this.exportHandleDrag;
    this.exportHandleDrag = undefined;
    if (drag && drag.element.hasPointerCapture?.(drag.pointerId)) {
      drag.element.releasePointerCapture(drag.pointerId);
    }
  }

  async exportMp4(): Promise<void> {
    if (
      !this.cameraName ||
      this.exportRunning ||
      this.exportValidationError !== null ||
      this.exportStart === null ||
      this.exportEnd === null
    ) {
      return;
    }
    const abortController = new AbortController();
    this.exportAbort = abortController;
    this.exportRunning = true;
    this.exportError = null;
    this.exportNotice = null;
    this.exportProgress = {
      stage: "resolving",
      completedFragments: 0,
      totalFragments: 0,
    };
    this.changeDetector.markForCheck();

    try {
      const result = await this.runClipExport({
        cameraName: this.cameraName,
        startTimeMs: this.exportStart,
        endTimeMs: this.exportEnd,
        recordings: this.recordings,
        store: this.controller.recordingStore,
        signal: abortController.signal,
        onProgress: (progress) => {
          if (this.exportAbort === abortController) {
            this.exportProgress = progress;
            this.changeDetector.markForCheck();
          }
        },
      });
      if (this.exportAbort !== abortController) return;
      this.downloadBlob(result.blob, result.filename);
      this.exportNotice = result.audioOmittedReason ?? null;
    } catch (error) {
      if (
        abortController.signal.aborted ||
        error instanceof ClipExportCancelledError
      ) {
        return;
      }
      this.exportError =
        error instanceof Error ? error.message : "Could not export clip.";
    } finally {
      if (this.exportAbort === abortController) {
        this.exportAbort = undefined;
        this.exportRunning = false;
        this.exportProgress = null;
        this.changeDetector.markForCheck();
      }
    }
  }

  cancelExport(): void {
    this.exportAbort?.abort();
  }

  downloadBlob(blob: Blob, filename: string): void {
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), BLOB_REVOKE_DELAY_MS);
  }

  private toLocalDateTimeInput(time: number | null): string {
    if (time === null || !Number.isFinite(time)) return "";
    const date = new Date(time);
    const pad = (value: number) => value.toString().padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  }

  private clampTime(time: number): number {
    return Math.max(this.windowStart, Math.min(time, this.windowEnd - 1));
  }

  seek(time: number): void {
    this.endDrag();
    void this.controller.seek(this.clampTime(time));
  }

  seekPointer(event: PointerEvent): void {
    if (
      event.button !== 0 ||
      this.dragPointer !== undefined ||
      this.exportHandleDrag !== undefined
    )
      return;
    const element = event.currentTarget as HTMLElement;
    // Preventing the pointer default below also prevents its normal focus action.
    element.focus({ preventScroll: true });
    this.dragPointer = event.pointerId;
    this.dragElement = element;
    element.setPointerCapture?.(event.pointerId);
    this.lastPreviewAt = -Infinity;
    this.updateDrag(event);
    event.preventDefault?.();
  }

  movePointer(event: PointerEvent): void {
    if (this.dragPointer !== undefined && event.pointerId === this.dragPointer)
      this.updateDrag(event);
  }

  releasePointer(event: PointerEvent): void {
    if (this.dragPointer === undefined || event.pointerId !== this.dragPointer)
      return;
    if (event.type === "pointerup") this.updateDrag(event, false);
    const target = this.position;
    this.endDrag();
    // Always resolve the released selection, even inside the preview interval.
    void this.controller.seek(target);
  }

  private updateDrag(event: PointerEvent, preview = true): void {
    const bounds = this.dragElement!.getBoundingClientRect();
    if (!bounds.width) return;
    const target = this.clampTime(
      this.windowStart +
        ((event.clientX - bounds.left) / bounds.width) * this.windowDuration,
    );
    this.controller.selectTime(target);
    if (!preview) return;
    this.schedulePreview(target);
  }

  private schedulePreview(target: number): void {
    this.pendingPreviewTarget = target;
    if (this.previewTimer !== undefined) return;
    const delay = this.previewInterval - (Date.now() - this.lastPreviewAt);
    if (delay <= 0) this.requestPreview(target);
    else
      this.previewTimer = setTimeout(() => {
        this.previewTimer = undefined;
        const next = this.pendingPreviewTarget ?? this.position;
        this.pendingPreviewTarget = undefined;
        this.requestPreview(next);
      }, delay);
  }

  private requestPreview(target: number): void {
    this.lastPreviewAt = Date.now();
    void this.controller.seek(target);
  }

  private endDrag(): void {
    if (this.previewTimer !== undefined) clearTimeout(this.previewTimer);
    this.previewTimer = undefined;
    this.pendingPreviewTarget = undefined;
    const pointer = this.dragPointer;
    const element = this.dragElement;
    this.dragPointer = undefined;
    this.dragElement = undefined;
    if (pointer !== undefined && element?.hasPointerCapture?.(pointer)) {
      element.releasePointerCapture(pointer);
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

  speedChanged(value: string): void {
    this.setSpeed(Number(value) as PlaybackSpeed);
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

  onVisibilityChange(): void {
    if (document.hidden) this.controller.pause();
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
    this.cancelExport();
    this.endDrag();
    this.endExportHandleDrag();
    this.controller.destroy();
    this.routeSubscription?.unsubscribe();
    this.rangeRequest?.unsubscribe();
  }
}
