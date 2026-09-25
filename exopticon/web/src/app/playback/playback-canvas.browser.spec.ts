import { ActivatedRoute, convertToParamMap } from "@angular/router";
import { TestBed } from "@angular/core/testing";
import { of } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { PlaybackComponent } from "./playback.component";

describe("Playback canvas inspection", () => {
  it("draws the selected decoded frame and its exact successor", async () => {
    const response = await fetch("/base/test-fixtures/capture-session-a.mkv");
    expect(response.ok).toBe(true);
    const blob = await response.blob();
    const begin = Date.parse("2026-09-23T10:00:00Z");
    const descriptor: RecordingDescriptor = {
      fileId: 1,
      beginTime: new Date(begin).toISOString(),
      endTime: new Date(begin + 2000).toISOString(),
      byteLength: blob.size,
      contentUrl: "/fixture/content",
      nextUrl: "/fixture/next",
    };
    const recordings = jasmine.createSpyObj<RecordingService>("recordings", [
      "getRanges",
      "resolveAt",
      "download",
    ]);
    recordings.getRanges.and.returnValue(of({ ranges: [descriptor] }));
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(blob));
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
    const component = fixture.componentInstance;
    component.windowStart = begin;
    component.windowEnd = begin + component.windowDuration;
    fixture.detectChanges();
    try {
      await component.controller.seek(begin + 273);
      fixture.detectChanges();
      const canvas = fixture.nativeElement.querySelector(
        "canvas",
      ) as HTMLCanvasElement;
      expect(canvas.width).toBe(160);
      expect(canvas.height).toBe(120);
      const pixels = canvas
        .getContext("2d")!
        .getImageData(0, 0, canvas.width, canvas.height).data;
      expect(
        pixels.some((value, index) => index % 4 !== 3 && value !== 0),
      ).toBe(true);
      expect(component.controller.state.requestedTime).toBe(begin + 273);
      expect(component.displayedTime).toBe(begin + 323);
      expect(
        fixture.nativeElement.querySelector("time").textContent.trim(),
      ).not.toBe("");

      await component.controller.stepForward();
      fixture.detectChanges();
      expect(component.displayedTime).toBe(begin + 423);
    } finally {
      fixture.destroy();
    }
  });
});
