import { ChangeDetectorRef } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { ActivatedRoute, convertToParamMap, ParamMap } from "@angular/router";
import { of, Subject } from "rxjs";
import { RecordingRange, RecordingService } from "../recording.service";
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
      const date = root.querySelector<HTMLInputElement>(
        'input[type="datetime-local"]',
      )!;
      expect(date.required).toBe(true);
      expect(date.max).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/);
    } finally {
      fixture.destroy();
    }
  });
});
