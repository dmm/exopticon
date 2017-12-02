defmodule ExopticonWeb.V1.CameraView do
  use ExopticonWeb, :view

  def render("index.json", %{cameras: cameras}) do
    render_many(cameras, ExopticonWeb.V1.CameraView, "camera.json")
  end

  def render("show.json", %{camera: camera}) do
    render_one(camera, ExopticonWeb.V1.CameraView, "camera.json")
  end

  def render("camera.json", %{camera: camera}) do
    %{
      id: camera.id,
      name: camera.name,
      fps: camera.fps,
      type: camera.type,
      ptzType: camera.ptz_type
    }
  end
end
