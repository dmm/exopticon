import { fakeAsync, tick } from "@angular/core/testing";
import { of, Subject } from "rxjs";
import { ChangeDetectorRef } from "@angular/core";
import { ActivatedRoute, convertToParamMap, ParamMap } from "@angular/router";
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
    component.position = 1000;
    component.ranges = [
      { begin: 1000, end: 2000 },
      { begin: 3000, end: 4000 },
    ];
  });
  afterEach(() => component.ngOnDestroy());

  it("loads the camera from the URL and reloads on route changes", () => {
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

  it("steps forward only while paused and skips gaps", () => {
    component.stepForward();
    expect(component.position).toBeCloseTo(1000 + 1000 / 30);
    component.position = 1999;
    component.stepForward();
    expect(component.position).toBe(3000);
    component.playing = true;
    component.stepForward();
    expect(component.position).toBe(3000);
    component.pause();
    component.position = 3999;
    expect(component.canAdvance).toBe(false);
  });

  it("plays across gaps and stops at the last recording", fakeAsync(() => {
    // Control the monotonic clock independently of the test runner's fake timers.
    let now = 0;
    spyOn(performance, "now").and.callFake(() => now);
    component.position = 1990;
    component.togglePlay();
    now = 100;
    tick(100);
    expect(component.position).toBe(3000);
    now = 1200;
    tick(100);
    expect(component.playing).toBe(false);
  }));

  it("cancels stale requests when the window shifts", () => {
    const old = new Subject<{ ranges: RecordingRange[] }>();
    const current = new Subject<{ ranges: RecordingRange[] }>();
    recordings.getRanges.and.returnValues(old, current);
    component.loadRanges();
    component.shiftWindow(-1);
    expect(old.observers.length).toBe(0);
    expect(component.windowEnd - component.windowStart).toBe(
      component.windowDuration,
    );
    expect(recordings.getRanges).toHaveBeenCalledWith(
      "front",
      -1800000,
      1800000,
    );
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

  it("clears old availability and exposes request failures for retry", () => {
    const request = new Subject<{ ranges: RecordingRange[] }>();
    recordings.getRanges.and.returnValue(request);
    component.loadRanges();
    expect(component.ranges).toEqual([]);
    expect(component.canAdvance).toBe(false);
    request.error(new Error("offline"));
    expect(component.error).toBe(true);
    expect(component.loading).toBe(false);
    recordings.getRanges.and.returnValue(of({ ranges: [] }));
    component.loadRanges();
    expect(component.error).toBe(false);
  });

  it("seeks with touch coordinates and clamps keyboard seeks to the window", () => {
    component.seekPointer({
      button: 0,
      clientX: 150,
      currentTarget: {
        getBoundingClientRect: () => ({ left: 100, width: 200 }),
      },
    } as unknown as PointerEvent);
    expect(component.position).toBe(900000);
    component.seekKey(new KeyboardEvent("keydown", { key: "Home" }));
    component.seekKey(new KeyboardEvent("keydown", { key: "ArrowLeft" }));
    expect(component.position).toBe(0);
    component.seekKey(new KeyboardEvent("keydown", { key: "End" }));
    expect(component.position).toBe(component.windowEnd - 1);
  });
});
