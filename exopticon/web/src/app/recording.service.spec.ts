import { provideHttpClient } from "@angular/common/http";
import {
  HttpTestingController,
  provideHttpClientTesting,
} from "@angular/common/http/testing";
import { TestBed } from "@angular/core/testing";
import { RecordingService } from "./recording.service";

describe("RecordingService", () => {
  it("requests UTC ranges using the recording API contract", () => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    const http = TestBed.inject(HttpTestingController);
    let ranges: unknown;
    TestBed.inject(RecordingService)
      .getRanges("front door", 0, 3600000)
      .subscribe((value) => (ranges = value.ranges));
    const request = http.expectOne(
      (req) => req.url === "v1/recordings/front%20door",
    );
    expect(request.request.params.get("begin_time")).toBe(
      "1970-01-01T00:00:00.000Z",
    );
    expect(request.request.params.get("end_time")).toBe(
      "1970-01-01T01:00:00.000Z",
    );
    request.flush({ ranges: [] });
    expect(ranges).toEqual([]);
    http.verify();
  });
});
