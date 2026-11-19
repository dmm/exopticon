import { provideHttpClient } from "@angular/common/http";
import {
  HttpTestingController,
  provideHttpClientTesting,
} from "@angular/common/http/testing";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { RouterTestingHarness } from "@angular/router/testing";
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
