defmodule ExopticonWeb.V1.FileView do
  use ExopticonWeb, :view

  def render("index.json", %{files: files}) do
    render_many(files, ExopticonWeb.V1.FileView, "file.json")
  end

  def render("file.json", %{file: f}) do
    %{
      camera_id: f.camera_id,
      begin_time: f.begin_time,
      end_time: f.end_time,
      begin_monotonic: f.begin_monotonic,
      end_monotonic: f.end_monotonic,
      monotonic_index: f.monotonic_index,
      filename: f.filename,
      size: f.size
      }
  end
end
