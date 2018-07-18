defmodule ExopticonWeb.V1.VideoUnitView do
  use ExopticonWeb, :view
  alias ExopticonWeb.V1.VideoUnitView

  def render("index.json", %{video_units: video_units}) do
    render_many(video_units, VideoUnitView, "video_unit.json")
  end

  def render("show.json", %{video_unit: video_unit}) do
    %{data: render_one(video_unit, VideoUnitView, "video_unit.json")}
  end

  def render("video_unit.json", %{video_unit: video_unit}) do
    %{
      id: video_unit.id,
      begin_time: video_unit.begin_time,
      end_time: video_unit.end_time,
      begin_monotonic: video_unit.begin_monotonic,
      end_monotonic: video_unit.end_monotonic,
      monotonic_index: video_unit.monotonic_index,
      files: render_many(video_unit.files, ExopticonWeb.V1.FileView, "file.json")
    }
  end
end
