import { CameraGroup } from "./camera-group";

describe("CameraGroup", () => {
  it("should create an instance", () => {
    const cameraGroup: CameraGroup = {
      metadata: { name: "all", displayName: "All cameras" },
      spec: { members: [] },
      status: {},
    };

    expect(cameraGroup).toBeTruthy();
  });
});
