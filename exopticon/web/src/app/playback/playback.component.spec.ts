import { ChangeDetectorRef } from "@angular/core";
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
    expect(seek).toHaveBeenCalledWith(900000);
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
});
