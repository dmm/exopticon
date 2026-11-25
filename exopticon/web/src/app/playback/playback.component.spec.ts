import { formatDate } from "@angular/common";
import { ChangeDetectorRef } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { ActivatedRoute, convertToParamMap, ParamMap } from "@angular/router";
import { of, Subject } from "rxjs";
import { RecordingRange, RecordingService } from "../recording.service";
import {
  ClipExportCancelledError,
  ClipExportError,
  ClipExportOptions,
  ClipExportResult,
  MAX_CLIP_DURATION_MS,
} from "./clip-exporter";
import { PlaybackComponent } from "./playback.component";

describe("PlaybackComponent", () => {
  let component: PlaybackComponent;
  let params: Subject<ParamMap>;
  let recordings: jasmine.SpyObj<RecordingService>;
  beforeEach(() => {
    recordings = jasmine.createSpyObj("RecordingService", ["getRanges"]);
    recordings.getRanges.and.returnValue(of({ ranges: [] }));
    params = new Subject<ParamMap>();
    component = new PlaybackComponent(
      { paramMap: params } as unknown as ActivatedRoute,
      recordings,
      { markForCheck: () => {} } as ChangeDetectorRef,
    );
    component.cameraName = "front";
    component.windowStart = 0;
    component.windowEnd = component.windowDuration;
  });
  afterEach(() => component.ngOnDestroy());

  it("pauses hidden playback and stays paused when visible again", async () => {
    await TestBed.configureTestingModule({
      imports: [PlaybackComponent],
      providers: [
        { provide: ActivatedRoute, useValue: { paramMap: params } },
        { provide: RecordingService, useValue: recordings },
      ],
    }).compileComponents();
    const fixture = TestBed.createComponent(PlaybackComponent);
    const controller = fixture.componentInstance.controller;
    const hidden = spyOnProperty(document, "hidden", "get");
    const pause = spyOn(controller, "pause").and.callThrough();
    const play = spyOn(controller, "play");
    try {
      fixture.detectChanges();
      for (const phase of ["playing", "buffering"] as const) {
        controller.state = {
          phase,
          requestedTime: 1500,
          displayedTime: 1200,
          error: null,
        };
        hidden.and.returnValue(false);
        document.dispatchEvent(new Event("visibilitychange"));
        expect(controller.state.phase).toBe(phase);

        hidden.and.returnValue(true);
        document.dispatchEvent(new Event("visibilitychange"));
        expect(controller.state.phase).toBe("paused");
        expect(controller.state.requestedTime).toBe(1200);
        expect(controller.state.displayedTime).toBe(1200);

        hidden.and.returnValue(false);
        document.dispatchEvent(new Event("visibilitychange"));
        expect(controller.state.phase).toBe("paused");
        expect(play).not.toHaveBeenCalled();
      }
    } finally {
      fixture.destroy();
    }
    pause.calls.reset();
    hidden.and.returnValue(true);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(pause).not.toHaveBeenCalled();
  });

  it("loads the camera from the URL and releases route work", () => {
    component.ngOnInit();
    params.next(convertToParamMap({ camera_name: "front" }));
    expect(recordings.getRanges).toHaveBeenCalledWith(
      "front",
      0,
      component.windowDuration,
    );
    params.next(convertToParamMap({ camera_name: "back" }));
    expect(component.cameraName).toBe("back");
    expect(recordings.getRanges).toHaveBeenCalledWith(
      "back",
      0,
      component.windowDuration,
    );
    component.ngOnDestroy();
    expect(params.observers.length).toBe(0);
  });

  it("keeps the displayed timestamp while navigating timeline windows", () => {
    component.controller.state = {
      phase: "paused",
      requestedTime: 1200,
      displayedTime: 1300,
      error: null,
    };
    component.shiftWindow(-1);
    expect(component.controller.state.displayedTime).toBe(1300);
    expect(component.controller.state.requestedTime).toBe(1200);
    expect(component.windowEnd - component.windowStart).toBe(
      component.windowDuration,
    );
  });

  it("cancels stale range requests when the window shifts", () => {
    const old = new Subject<{ ranges: RecordingRange[] }>();
    const current = new Subject<{ ranges: RecordingRange[] }>();
    recordings.getRanges.and.returnValues(old, current);
    component.loadRanges();
    component.shiftWindow(-1);
    expect(old.observers.length).toBe(0);
    expect(recordings.getRanges).toHaveBeenCalledWith("front", -900000, 900000);
    current.next({
      ranges: [
        {
          beginTime: new Date(0).toISOString(),
          endTime: new Date(1000).toISOString(),
        },
      ],
    });
    expect(component.ranges).toEqual([{ begin: 0, end: 1000 }]);
    expect(component.loading).toBe(false);
  });

  it("maps pointer and keyboard seeks to requested UTC positions", () => {
    const seek = spyOn(component.controller, "seek").and.returnValue(
      Promise.resolve(),
    );
    component.seekPointer({
      button: 0,
      clientX: 150,
      currentTarget: {
        focus: () => {},
        getBoundingClientRect: () => ({ left: 100, width: 200 }),
      },
    } as unknown as PointerEvent);
    expect(seek).toHaveBeenCalledWith(450000);
    component.controller.state = {
      phase: "paused",
      requestedTime: 1000,
      displayedTime: 1200,
      error: null,
    };
    component.seekKey(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    expect(seek).toHaveBeenCalledWith(2000);
    component.seekKey(new KeyboardEvent("keydown", { key: "End" }));
    expect(seek).toHaveBeenCalledWith(component.windowEnd - 1);
  });
  it("bounds drag previews, moves the selection immediately and resolves the release", () => {
    jasmine.clock().install();
    jasmine.clock().mockDate(new Date(10000));
    try {
      const seek = spyOn(component.controller, "seek").and.resolveTo();
      spyOn(component.controller, "selectTime").and.callFake((time) => {
        component.controller.state.requestedTime = time;
      });
      const element = {
        focus: () => {},
        getBoundingClientRect: () => ({ left: 0, width: 1800 }),
        setPointerCapture: jasmine.createSpy(),
        hasPointerCapture: () => true,
        releasePointerCapture: jasmine.createSpy(),
      };
      const pointer = (x: number, type = "pointermove", pointerId = 1) =>
        ({
          button: 0,
          clientX: x,
          pointerId,
          type,
          currentTarget: element,
        }) as unknown as PointerEvent;
      component.seekPointer(pointer(1, "pointerdown"));
      expect(seek).toHaveBeenCalledTimes(1);
      expect(seek).toHaveBeenCalledWith(1000);
      component.movePointer(pointer(2));
      component.movePointer(pointer(3));
      expect(component.position).toBe(3000);
      expect(seek).toHaveBeenCalledTimes(1);
      component.movePointer(pointer(99, "pointermove", 2));
      expect(component.position).toBe(3000);
      jasmine.clock().tick(150);
      expect(seek).toHaveBeenCalledWith(3000);
      component.movePointer(pointer(4));
      component.releasePointer(pointer(5, "pointerup"));
      expect(seek).toHaveBeenCalledTimes(3);
      expect(seek.calls.mostRecent().args).toEqual([5000]);
      jasmine.clock().tick(500);
      expect(seek).toHaveBeenCalledTimes(3);
      expect(element.releasePointerCapture).toHaveBeenCalledWith(1);
    } finally {
      jasmine.clock().uninstall();
    }
  });

  it("finishes canceled drags and removes scheduled previews on destruction", () => {
    jasmine.clock().install();
    try {
      const seek = spyOn(component.controller, "seek").and.resolveTo();
      spyOn(component.controller, "selectTime").and.callFake((time) => {
        component.controller.state.requestedTime = time;
      });
      const event = (x: number) =>
        ({
          button: 0,
          pointerId: 1,
          clientX: x,
          currentTarget: {
            focus: () => {},
            getBoundingClientRect: () => ({ left: 0, width: 1800 }),
          },
        }) as unknown as PointerEvent;
      component.seekPointer(event(1));
      component.movePointer(event(2));
      component.releasePointer(event(2));
      expect(seek.calls.mostRecent().args).toEqual([2000]);
      component.seekPointer(event(3));
      component.movePointer(event(4));
      component.ngOnDestroy();
      const count = seek.calls.count();
      jasmine.clock().tick(500);
      expect(seek.calls.count()).toBe(count);
    } finally {
      jasmine.clock().uninstall();
    }
  });

  it("zooms around the visible playhead without changing playback", () => {
    component.controller.state = {
      phase: "playing",
      requestedTime: 600000,
      displayedTime: 600050,
      error: null,
    };
    const seek = spyOn(component.controller, "seek");
    component.setScale(3);
    expect(component.windowStart).toBe(300000);
    expect(component.windowEnd).toBe(900000);
    expect(component.controller.state.phase).toBe("playing");
    expect(component.displayedTime).toBe(600050);
    expect(seek).not.toHaveBeenCalled();
    expect(recordings.getRanges).toHaveBeenCalledWith("front", 300000, 900000);
  });

  it("uses the window midpoint without a visible selection", () => {
    expect(component.playheadVisible).toBe(false);
    component.setScale(4);
    expect(component.windowStart).toBe(870000);
    expect(component.windowEnd).toBe(930000);
    component.controller.state.requestedTime = 0;
    component.setScale(3);
    expect(component.windowStart).toBe(600000);
    expect(component.windowEnd).toBe(1200000);
  });

  it("limits zooming at both ends and keeps windows out of the future", () => {
    spyOn(Date, "now").and.returnValue(1800000);
    component.setScale(0);
    expect(component.windowEnd).toBe(1800000);
    expect(component.windowDuration).toBe(86400000);
    recordings.getRanges.calls.reset();
    component.setScale(-1);
    expect(recordings.getRanges).not.toHaveBeenCalled();
    component.setScale(4);
    recordings.getRanges.calls.reset();
    component.setScale(5);
    expect(recordings.getRanges).not.toHaveBeenCalled();
  });

  it("moves by half the selected span and preserves it for Latest", () => {
    spyOn(Date, "now").and.returnValue(3600000);
    component.setScale(4);
    const start = component.windowStart;
    component.shiftWindow(-1);
    expect(component.windowStart).toBe(start - 30000);
    expect(component.shiftLabel).toBe("30 sec");
    component.latest();
    expect(component.windowStart).toBe(3540000);
    expect(component.windowEnd).toBe(3600000);
  });

  it("aligns ticks to clock boundaries and shows seconds at the minute scale", () => {
    component.setScale(4);
    component.windowStart = new Date(2026, 0, 1, 12, 0, 3).getTime();
    component.windowEnd = component.windowStart + component.windowDuration;
    expect(component.ticks.length).toBe(6);
    expect(new Date(component.ticks[0]).getSeconds()).toBe(10);
    expect(component.tickFormat).toBe("HH:mm:ss");
    component.setScale(1);
    expect(component.tickFormat).toBe("HH:mm");
    expect(
      component.ticks.every((tick) => new Date(tick).getMinutes() % 10 === 0),
    ).toBe(true);
  });

  it("opens and seeks a chosen local date/time while rejecting invalid or future input", () => {
    const time = new Date(2026, 0, 1, 12, 0).getTime();
    spyOn(Date, "now").and.returnValue(time + 3600000);
    const seek = spyOn(component.controller, "seek").and.returnValue(
      Promise.resolve(),
    );
    component.jumpToTime("2026-01-01T12:00:00");
    expect((component.windowStart + component.windowEnd) / 2).toBe(time);
    expect(seek).toHaveBeenCalledWith(time);
    seek.calls.reset();
    component.jumpToTime("invalid");
    component.jumpToTime("2027-01-01T12:00:00");
    expect(seek).not.toHaveBeenCalled();
  });

  it("connects preset buttons to the visible scale", async () => {
    await TestBed.configureTestingModule({
      imports: [PlaybackComponent],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap: of(convertToParamMap({ camera_name: "front" })),
          },
        },
        { provide: RecordingService, useValue: recordings },
      ],
    }).compileComponents();
    const fixture = TestBed.createComponent(PlaybackComponent);
    try {
      fixture.detectChanges();
      const root: HTMLElement = fixture.nativeElement;
      const presets = root.querySelectorAll<HTMLButtonElement>(
        ".scale-presets button",
      );
      expect(presets[2].getAttribute("aria-pressed")).toBe("true");
      presets[4].click();
      fixture.detectChanges();
      expect(fixture.componentInstance.windowDuration).toBe(60000);
      expect(presets[4].getAttribute("aria-pressed")).toBe("true");
      presets[3].click();
      fixture.detectChanges();
      expect(presets[3].getAttribute("aria-pressed")).toBe("true");
      fixture.componentInstance.controller.state = {
        phase: "seeking",
        requestedTime: 1000,
        displayedTime: 5000,
        error: null,
      };
      fixture.componentInstance.setSpeed(1);
      fixture.detectChanges();
      const times = root.querySelectorAll(".frame-details time");
      expect(times.length).toBe(1);
      expect(times[0].getAttribute("aria-label")).toBe("Displayed frame time");
      expect(root.querySelector(".frame-details")!.textContent).not.toContain(
        "Selected time:",
      );
      expect(root.querySelector(".preview-status")!.textContent).toContain(
        "Seeking…",
      );
      const selectedTime = root.querySelector(".selected-time time")!;
      expect(selectedTime.getAttribute("aria-label")).toBe("Selected time");
      expect(selectedTime.textContent!.trim()).toBe(
        formatDate(1000, "MMM d, y HH:mm:ss.SSS z", "en-US"),
      );
      const timeline = root.querySelector<HTMLElement>(".timeline")!;
      const timelineTop =
        timeline.getBoundingClientRect().top - root.getBoundingClientRect().top;
      expect(root.querySelector('[role="tooltip"]')).toBeNull();
      const summary = root.querySelector<HTMLElement>(".preview-summary")!;
      expect(summary.querySelector("time")).not.toBeNull();
      expect(summary.querySelector(".preview-status")).not.toBeNull();
      expect(summary.querySelector(".selected-time")).not.toBeNull();
      fixture.componentInstance.controller.state.phase = "paused";
      fixture.componentInstance.setSpeed(1);
      fixture.detectChanges();
      expect(
        timeline.getBoundingClientRect().top - root.getBoundingClientRect().top,
      ).toBe(timelineTop);
      fixture.componentInstance.controller.state.displayedTime = 1050;
      fixture.componentInstance.setSpeed(1);
      fixture.detectChanges();
      expect(selectedTime.textContent!.trim()).toBe(
        formatDate(1000, "MMM d, y HH:mm:ss.SSS z", "en-US"),
      );
      fixture.componentInstance.controller.selectTime(2000);
      fixture.detectChanges();
      expect(selectedTime.textContent!.trim()).toBe(
        formatDate(2000, "MMM d, y HH:mm:ss.SSS z", "en-US"),
      );
      expect(times[0].textContent!.trim()).toBe(
        formatDate(1050, "MMM d, y HH:mm:ss.SSS z", "en-US"),
      );
      expect(
        timeline.getBoundingClientRect().top - root.getBoundingClientRect().top,
      ).toBe(timelineTop);
      root.style.width = "380px";
      const narrowTop =
        timeline.getBoundingClientRect().top - root.getBoundingClientRect().top;
      expect(summary.getBoundingClientRect().height).toBeLessThan(60);
      fixture.componentInstance.controller.state.displayedTime = 5000;
      fixture.componentInstance.setSpeed(1);
      fixture.detectChanges();
      expect(
        timeline.getBoundingClientRect().top - root.getBoundingClientRect().top,
      ).toBe(narrowTop);
      expect(summary.getBoundingClientRect().height).toBeLessThan(60);
      root.style.width = "700px";
      const transport = root.querySelector<HTMLElement>(".transport")!;
      const controls = Array.from(
        transport.querySelectorAll<HTMLButtonElement>("button"),
      );
      expect(controls.map((button) => button.textContent!.trim())).toEqual([
        "−1s",
        "Play",
        "Next frame",
        "+1s",
      ]);
      expect(
        controls.every(
          (button) =>
            button.getBoundingClientRect().top ===
            controls[0].getBoundingClientRect().top,
        ),
      ).toBe(true);
      const speed = transport.querySelector<HTMLSelectElement>("select")!;
      speed.value = "2";
      speed.dispatchEvent(new Event("change"));
      expect(fixture.componentInstance.controller.speed).toBe(2);
      expect(speed.getBoundingClientRect().top).toBe(
        controls[1].getBoundingClientRect().top,
      );
      expect(
        speed.getBoundingClientRect().left -
          controls[3].getBoundingClientRect().right,
      ).toBeGreaterThanOrEqual(20);
      transport.style.width = "180px";
      expect(transport.scrollWidth).toBeLessThanOrEqual(transport.clientWidth);
      expect(speed.getBoundingClientRect().top).toBeGreaterThanOrEqual(
        Math.max(
          ...controls.map((button) => button.getBoundingClientRect().bottom),
        ),
      );
      expect(
        Math.abs(
          (speed.getBoundingClientRect().left +
            speed.getBoundingClientRect().right) /
            2 -
            (transport.getBoundingClientRect().left +
              transport.getBoundingClientRect().right) /
              2,
        ),
      ).toBeLessThan(1);
      const seek = spyOn(
        fixture.componentInstance.controller,
        "seek",
      ).and.callFake(async (time) => {
        fixture.componentInstance.controller.state.requestedTime = time;
      });
      spyOn(timeline, "setPointerCapture");
      const bounds = timeline.getBoundingClientRect();
      const pointer = {
        button: 0,
        pointerId: 1,
        clientX: bounds.left + bounds.width / 2,
        bubbles: true,
        cancelable: true,
      };
      timeline.dispatchEvent(new PointerEvent("pointerdown", pointer));
      expect(document.activeElement).toBe(timeline);
      timeline.dispatchEvent(new PointerEvent("pointerup", pointer));
      const selected = fixture.componentInstance.position;
      timeline.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
      );
      expect(seek.calls.mostRecent().args).toEqual([selected + 1000]);
      const date = root.querySelector<HTMLInputElement>(
        'input[type="datetime-local"]',
      )!;
      expect(date.required).toBe(true);
      expect(date.max).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/);
    } finally {
      fixture.destroy();
    }
  });

  it("initializes clip export mode around the playhead and clamps to the enclosing range", () => {
    component.ranges = [{ begin: 100_000, end: 120_000 }];
    component.controller.state = {
      phase: "paused",
      requestedTime: 105_400,
      displayedTime: 105_400,
      error: null,
    };

    component.toggleExportMode();
    expect(component.exportMode).toBe(true);
    expect(component.exportStart).toBe(105_000);
    expect(component.exportEnd).toBe(120_000);
    expect(component.exportValidationError).toBeNull();
    expect(component.exportDurationLabel).toBe("0:15 / 5:00 max");

    component.toggleExportMode();
    expect(component.exportMode).toBe(false);
    expect(component.exportStart).toBeNull();
    expect(component.exportEnd).toBeNull();
  });

  it("sets start/end from playhead with 5-minute clamping and detects gaps inline", () => {
    component.ranges = [
      { begin: 100_000, end: 500_000 },
      { begin: 600_000, end: 900_000 },
    ];
    component.toggleExportMode();

    component.controller.state.requestedTime = 120_000;
    component.setExportStartFromPlayhead();
    component.controller.state.requestedTime = 500_000;
    component.setExportEndFromPlayhead();
    // 500_000 - 120_000 = 380s > 300s -> clamped to 5 minutes ending at 500_000.
    expect(component.exportEnd).toBe(500_000);
    expect(component.exportStart).toBe(500_000 - MAX_CLIP_DURATION_MS);
    expect(component.exportValidationError).toBeNull();

    // Crossing the gap between 500_000 and 600_000 triggers inline validation error.
    component.exportStart = 450_000;
    component.exportEnd = 650_000;
    expect(component.exportValidationError).toContain("gap");

    // Exceeding 5 minutes via direct input triggers duration error.
    component.exportStart = 100_000;
    component.exportEnd = 100_000 + MAX_CLIP_DURATION_MS + 1000;
    expect(component.exportValidationError).toContain("5 minutes");
  });

  it("drags export handles with throttled boundary frame previews and enforces 5-minute cap", () => {
    jasmine.clock().install();
    jasmine.clock().mockDate(new Date(10_000));
    try {
      component.ranges = [{ begin: 0, end: 1_800_000 }];
      component.controller.state = {
        phase: "paused",
        requestedTime: 600_000,
        displayedTime: 600_000,
        error: null,
      };
      component.toggleExportMode();
      expect(component.exportStart).toBe(600_000);
      expect(component.exportEnd).toBe(630_000);

      const seek = spyOn(component.controller, "seek").and.resolveTo();
      const selectTime = spyOn(component.controller, "selectTime").and.callFake(
        (time) => {
          component.controller.state.requestedTime = time;
        },
      );
      const timeline = {
        getBoundingClientRect: () => ({ left: 0, width: 1800 }),
      } as HTMLElement;
      const handleEl = {
        parentElement: timeline,
        focus: () => {},
        setPointerCapture: jasmine.createSpy(),
        hasPointerCapture: () => true,
        releasePointerCapture: jasmine.createSpy(),
      } as unknown as HTMLElement;
      const pointer = (x: number, type = "pointermove", pointerId = 7) =>
        ({
          button: 0,
          clientX: x,
          pointerId,
          type,
          currentTarget: handleEl,
          stopPropagation: jasmine.createSpy(),
          preventDefault: jasmine.createSpy(),
        }) as unknown as PointerEvent;

      // Drag end handle past 5 minutes (1200s = 1_200_000 ms -> clamped to 600_000 + 300_000 = 900_000).
      component.startExportHandlePointer("end", pointer(1200, "pointerdown"));
      expect(component.exportEnd).toBe(900_000);
      expect(selectTime).toHaveBeenCalledWith(900_000);
      expect(seek).toHaveBeenCalledTimes(1);
      expect(seek).toHaveBeenCalledWith(900_000);

      // Drag within throttle window queues preview.
      component.moveExportHandlePointer(pointer(700));
      expect(component.exportEnd).toBe(700_000);
      expect(seek).toHaveBeenCalledTimes(1);
      jasmine.clock().tick(150);
      expect(seek).toHaveBeenCalledWith(700_000);

      // Release resolves final boundary frame.
      component.releaseExportHandlePointer(pointer(750, "pointerup"));
      expect(component.exportEnd).toBe(750_000);
      expect(seek.calls.mostRecent().args).toEqual([750_000]);
      expect(handleEl.releasePointerCapture).toHaveBeenCalledWith(7);

      // Keyboard step on start handle moves by 10s with Shift and previews.
      component.exportHandleKey(
        "start",
        new KeyboardEvent("keydown", { key: "ArrowLeft", shiftKey: true }),
      );
      expect(component.exportStart).toBe(590_000);
      expect(seek.calls.mostRecent().args).toEqual([590_000]);
    } finally {
      jasmine.clock().uninstall();
    }
  });

  it("runs MP4 export with progress, downloads result, handles errors, and cancels on destroy or cancel", async () => {
    component.ranges = [{ begin: 100_000, end: 200_000 }];
    component.toggleExportMode();
    component.exportStart = 110_000;
    component.exportEnd = 140_000;

    const downloadSpy = spyOn(component, "downloadBlob");
    let capturedOptions: ClipExportOptions | undefined;
    component.runClipExport = async (options) => {
      capturedOptions = options;
      options.onProgress?.({
        stage: "downloading",
        completedFragments: 0,
        totalFragments: 2,
      });
      expect(component.exportRunning).toBe(true);
      expect(component.exportProgressText).toBe("Downloading fragment 1 of 2…");
      return {
        blob: new Blob(["mp4"], { type: "video/mp4" }),
        filename: "front-clip.mp4",
        actualStartMs: 110_000,
        actualEndMs: 140_000,
        videoPacketCount: 10,
        audioPacketCount: 0,
        audioOmittedReason: "Legacy audio skipped",
      } as ClipExportResult;
    };

    await component.exportMp4();
    expect(capturedOptions?.cameraName).toBe("front");
    expect(capturedOptions?.startTimeMs).toBe(110_000);
    expect(capturedOptions?.endTimeMs).toBe(140_000);
    expect(downloadSpy).toHaveBeenCalledWith(
      jasmine.any(Blob),
      "front-clip.mp4",
    );
    expect(component.exportRunning).toBe(false);
    expect(component.exportError).toBeNull();
    expect(component.exportNotice).toBe("Legacy audio skipped");

    // Error handling displays message without triggering download.
    downloadSpy.calls.reset();
    component.runClipExport = async () => {
      throw new ClipExportError("Selected range contains a recording gap.");
    };
    await component.exportMp4();
    expect(downloadSpy).not.toHaveBeenCalled();
    expect(component.exportError).toBe(
      "Selected range contains a recording gap.",
    );

    // Cancellation aborts the signal and clears running state without showing an error.
    let aborted = false;
    component.runClipExport = (options) =>
      new Promise<ClipExportResult>((_resolve, reject) => {
        options.signal?.addEventListener("abort", () => {
          aborted = true;
          reject(new ClipExportCancelledError());
        });
      });
    const pending = component.exportMp4();
    expect(component.exportRunning).toBe(true);
    component.cancelExport();
    await pending;
    expect(aborted).toBe(true);
    expect(component.exportRunning).toBe(false);
    expect(component.exportError).toBeNull();
  });

  it("renders clip export controls and timeline handles when Export Clip is toggled", async () => {
    recordings.getRanges.and.returnValue(
      of({
        ranges: [
          {
            beginTime: new Date(Date.now() - 120_000).toISOString(),
            endTime: new Date(Date.now() - 10_000).toISOString(),
          },
        ],
      }),
    );
    await TestBed.configureTestingModule({
      imports: [PlaybackComponent],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap: of(convertToParamMap({ camera_name: "front" })),
          },
        },
        { provide: RecordingService, useValue: recordings },
      ],
    }).compileComponents();
    const fixture = TestBed.createComponent(PlaybackComponent);
    try {
      fixture.detectChanges();
      const root: HTMLElement = fixture.nativeElement;
      expect(root.querySelector(".clip-export-panel")).toBeNull();
      expect(root.querySelector(".export-selection")).toBeNull();

      const toggle = root.querySelector<HTMLButtonElement>(
        ".export-clip-toggle",
      )!;
      toggle.click();
      fixture.detectChanges();

      expect(toggle.getAttribute("aria-pressed")).toBe("true");
      expect(root.querySelector(".clip-export-panel")).not.toBeNull();
      expect(root.querySelector(".export-selection")).not.toBeNull();
      expect(root.querySelector(".export-handle-start")).not.toBeNull();
      expect(root.querySelector(".export-handle-end")).not.toBeNull();
      const exportButton =
        root.querySelector<HTMLButtonElement>(".export-mp4-button")!;
      expect(exportButton.disabled).toBe(false);
    } finally {
      fixture.destroy();
    }
  });

  it("rounds sub-millisecond range begin times upward while flooring end times", () => {
    recordings.getRanges.and.returnValue(
      of({
        ranges: [
          {
            beginTime: "1970-01-01T00:00:00.123456Z",
            endTime: "1970-01-01T00:00:10.789456Z",
          },
          {
            beginTime: "1970-01-01T00:00:20.000000Z",
            endTime: "1970-01-01T00:00:30.000000Z",
          },
        ],
      }),
    );

    component.loadRanges();

    expect(component.ranges).toEqual([
      { begin: 124, end: 10789 },
      { begin: 20000, end: 30000 },
    ]);
  });

  it("aborts in-flight exports on camera switch and component destroy", async () => {
    component.ngOnInit();
    params.next(convertToParamMap({ camera_name: "front" }));
    component.ranges = [{ begin: 100_000, end: 200_000 }];
    component.toggleExportMode();
    component.exportStart = 110_000;
    component.exportEnd = 140_000;

    let cameraSwitchAborted = false;
    component.runClipExport = (options) =>
      new Promise<ClipExportResult>((_resolve, reject) => {
        options.signal?.addEventListener("abort", () => {
          cameraSwitchAborted = true;
          reject(new ClipExportCancelledError());
        });
      });

    const firstExport = component.exportMp4();
    expect(component.exportRunning).toBe(true);
    params.next(convertToParamMap({ camera_name: "back" }));
    await firstExport;
    expect(cameraSwitchAborted).toBe(true);
    expect(component.exportMode).toBe(false);
    expect(component.exportRunning).toBe(false);

    // Start another export on the new camera and destroy the component.
    component.ranges = [{ begin: 100_000, end: 200_000 }];
    component.toggleExportMode();
    component.exportStart = 110_000;
    component.exportEnd = 140_000;
    let destroyAborted = false;
    component.runClipExport = (options) =>
      new Promise<ClipExportResult>((_resolve, reject) => {
        options.signal?.addEventListener("abort", () => {
          destroyAborted = true;
          reject(new ClipExportCancelledError());
        });
      });

    const secondExport = component.exportMp4();
    expect(component.exportRunning).toBe(true);
    component.ngOnDestroy();
    await secondExport;
    expect(destroyAborted).toBe(true);
    expect(component.exportRunning).toBe(false);
  });
});
