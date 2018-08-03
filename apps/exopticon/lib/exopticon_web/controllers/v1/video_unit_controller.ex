defmodule ExopticonWeb.V1.VideoUnitController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video

  action_fallback(ExopticonWeb.FallbackController)

  def index(conn, %{"camera_id" => camera_id}) do
    video_units = Video.list_video_units_by_camera(camera_id)
    render(conn, "index.json", video_units: video_units)
  end

  def show(conn, %{"id" => id}) do
    video_unit = Video.get_video_unit!(id)
    render(conn, "show.json", video_unit: video_unit)
  end

  def between(conn, %{"camera_id" => id, "begin_time" => begin_time, "end_time" => end_time}) do
    videos = Video.list_video_units_between(id, begin_time, end_time)

    render(conn, "index.json", video_units: videos)
  end
end
