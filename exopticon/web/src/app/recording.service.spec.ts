import { provideHttpClient } from "@angular/common/http";
import {
  HttpTestingController,
  provideHttpClientTesting,
} from "@angular/common/http/testing";
import { TestBed } from "@angular/core/testing";
import { RecordingDescriptor, RecordingService } from "./recording.service";

const descriptor: RecordingDescriptor = {
  fileId: 42,
  beginTime: "2026-09-23T10:00:00Z",
  endTime: "2026-09-23T10:00:10Z",
  byteLength: 4,
  contentUrl: "/v1/recordings/front%20door/files/42/content",
  nextUrl: "/v1/recordings/front%20door/files/42/next",
};

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

  it("resolves a time and follows the descriptor's successor URL", () => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    const http = TestBed.inject(HttpTestingController);
    const service = TestBed.inject(RecordingService);
    let resolved: RecordingDescriptor | undefined;
    service
      .resolveAt("front door", Date.parse("2026-09-23T10:00:05Z"))
      .subscribe((value) => (resolved = value));
    const resolve = http.expectOne(
      (req) => req.url === "v1/recordings/front%20door/samples",
    );
    expect(resolve.request.params.get("at")).toBe("2026-09-23T10:00:05.000Z");
    resolve.flush(descriptor);
    expect(resolved).toEqual(descriptor);

    let successor: RecordingDescriptor | null | undefined;
    service
      .getNext(descriptor)
      .subscribe((value) => (successor = value.recording));
    const next = http.expectOne(descriptor.nextUrl);
    expect(next.request.method).toBe("GET");
    next.flush({ recording: null });
    expect(successor).toBeNull();
    http.verify();
  });

  it("downloads a whole Blob and cancels the request on unsubscribe", () => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    const http = TestBed.inject(HttpTestingController);
    const service = TestBed.inject(RecordingService);
    const content = new Blob(["mkv!"], { type: "video/x-matroska" });
    let downloaded: Blob | undefined;
    service.download(descriptor).subscribe((value) => (downloaded = value));
    const request = http.expectOne(descriptor.contentUrl);
    expect(request.request.method).toBe("GET");
    expect(request.request.responseType).toBe("blob");
    expect(request.request.headers.has("Range")).toBe(false);
    request.flush(content);
    expect(downloaded).toBe(content);

    const subscription = service.download(descriptor).subscribe();
    const obsolete = http.expectOne(descriptor.contentUrl);
    subscription.unsubscribe();
    expect(obsolete.cancelled).toBe(true);
    http.verify();
  });
});
