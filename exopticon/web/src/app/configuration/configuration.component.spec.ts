import { provideHttpClient } from "@angular/common/http";
import {
  HttpTestingController,
  provideHttpClientTesting,
} from "@angular/common/http/testing";
import { TestBed } from "@angular/core/testing";
import { NavigationEnd, Router, provideRouter } from "@angular/router";
import { RouterTestingHarness } from "@angular/router/testing";
import { filter, take } from "rxjs/operators";
import { ConfigurationComponent } from "./configuration.component";

describe("Configuration request lifecycle", () => {
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([
          {
            path: "configuration/:resource/new",
            component: ConfigurationComponent,
            data: { mode: "new" },
          },
          {
            path: "configuration/:resource/:name/edit",
            component: ConfigurationComponent,
            data: { mode: "edit" },
          },
          {
            path: "configuration/:resource",
            component: ConfigurationComponent,
            data: { mode: "list" },
          },
        ]),
      ],
    });
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it("fills the camera form from an ONVIF profile and saves only on Create", async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl("/configuration/cameras/new");
    http
      .expectOne("/v1/storage_groups")
      .flush([{ metadata: { name: "primary", displayName: "Primary" } }]);
    harness.detectChanges();
    await harness.fixture.whenStable();
    const page = harness.routeNativeElement!;

    const setInput = (selector: string, value: string) => {
      const input = page.querySelector<HTMLInputElement>(selector)!;
      input.value = value;
      input.dispatchEvent(new Event("input", { bubbles: true }));
    };
    setInput('app-onvif-probe input[name="host"]', "camera.example");
    setInput('app-onvif-probe input[name="username"]', "camera@user");
    setInput('app-onvif-probe input[name="password"]', "pass%word:/");
    page.querySelector<HTMLButtonElement>("app-onvif-probe button")!.click();

    const probe = http.expectOne({
      method: "POST",
      url: "/v1/cameras/onvif/probe",
    });
    expect(probe.request.body).toEqual({
      host: "camera.example",
      port: 80,
      username: "camera@user",
      password: "pass%word:/",
    });
    probe.flush({
      host: "camera.example",
      port: 8080,
      deviceServiceUrl: "http://camera.example:8080/onvif/device_service",
      device: { model: "Test Camera" },
      profiles: [
        {
          name: "Main",
          token: "main",
          rtspUrl: "rtsp://camera.example/main",
          reuseCredentials: true,
          ptzSupported: true,
          snapshotUrl: "http://camera.example/snapshot",
          video: { codec: "H264", width: 1920, height: 1080, frameRate: 25 },
        },
        { name: "Sub", token: "sub", rtspUrl: null, ptzSupported: false },
      ],
    });
    harness.detectChanges();

    expect(page.textContent).toContain("http://camera.example/snapshot");
    expect(page.querySelector("app-onvif-probe img")).toBeNull();
    page
      .querySelector<HTMLButtonElement>(
        'button[aria-label="Add profile Main"]',
      )!
      .click();
    http.expectNone({ method: "POST", url: "/v1/cameras" });
    harness.detectChanges();
    await harness.fixture.whenStable();

    expect(page.querySelector("#rtspUrl")).toBeNull();
    expect(page.querySelector<HTMLInputElement>("#onvifDeviceUrl")!.value).toBe(
      "http://camera.example:8080/onvif/device_service",
    );
    expect(page.ownerDocument.activeElement?.id).toBe("resource-name");

    setInput("#resource-name", "test-camera");
    const storage = page.querySelector<HTMLSelectElement>("#storageGroupName")!;
    storage.value = "primary";
    storage.dispatchEvent(new Event("change", { bubbles: true }));
    page
      .querySelector<HTMLButtonElement>(
        "form.configuration-form button[type=submit]",
      )!
      .click();
    const save = http.expectOne({ method: "POST", url: "/v1/cameras" });
    expect(save.request.body.metadata).toEqual({ name: "test-camera" });
    expect(save.request.body.spec).toEqual(
      jasmine.objectContaining({
        storageGroupName: "primary",
        streamSource: {
          type: "onvif",
          deviceUrl: "http://camera.example:8080/onvif/device_service",
          profileToken: "main",
        },
        username: "camera@user",
        password: "pass%word:/",
      }),
    );
    expect(save.request.body.spec.rtspUrl).toBeUndefined();
    const router = TestBed.inject(Router);
    const navigation = router.events
      .pipe(
        filter((event) => event instanceof NavigationEnd),
        take(1),
      )
      .toPromise();
    save.flush(null, { status: 201, statusText: "Created" });
    await navigation;
    harness.detectChanges();
    http.expectOne({ method: "GET", url: "/v1/cameras" }).flush([]);
    expect(router.url).toBe("/configuration/cameras");
  });

  it("cancels both camera group reads when navigating to another group", async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl("/configuration/camera_groups/front/edit");
    const previousGroup = http.expectOne("/v1/camera_groups/front");
    const previousChoices = http.expectOne("/v1/cameras");

    const component = await harness.navigateByUrl(
      "/configuration/camera_groups/back/edit",
      ConfigurationComponent,
    );
    expect(previousGroup.cancelled).toBeTrue();
    expect(previousChoices.cancelled).toBeTrue();
    expect(component.loading).toBeTrue();

    http.expectOne("/v1/camera_groups/back").flush({
      metadata: { name: "back", displayName: "Back yard" },
      spec: { members: [] },
    });
    http.expectOne("/v1/cameras").flush([]);
    harness.detectChanges();
    await harness.fixture.whenStable();

    expect(
      harness.routeNativeElement?.querySelector<HTMLInputElement>(
        "input#display-name",
      )?.value,
    ).toBe("Back yard");
  });

  it("cancels an in-flight list read when leaving Configuration", async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl("/configuration/cameras");
    const cameras = http.expectOne("/v1/cameras");

    harness.fixture.destroy();

    expect(cameras.cancelled).toBeTrue();
  });

  it("lets a submitted save finish while navigation changes the active form", async () => {
    const harness = await RouterTestingHarness.create();
    const component = await harness.navigateByUrl(
      "/configuration/users/alice/edit",
      ConfigurationComponent,
    );
    http.expectOne("/v1/users/alice").flush({
      username: "alice",
      displayName: "Alice",
    });

    const save = component.save({ displayName: "Edited Alice" });
    const request = http.expectOne({ method: "PATCH", url: "/v1/users/alice" });
    await harness.navigateByUrl("/configuration/users/bob/edit");
    http.expectOne("/v1/users/bob").flush({
      username: "bob",
      displayName: "Bob",
    });

    expect(request.cancelled).toBeFalse();
    request.flush(null, { status: 204, statusText: "No Content" });
    await save;
    harness.detectChanges();
    await harness.fixture.whenStable();

    expect(
      harness.routeNativeElement?.querySelector<HTMLInputElement>(
        "input#display-name",
      )?.value,
    ).toBe("Bob");
  });
});
