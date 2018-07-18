defmodule ExopticonWeb.V1.VideoUnitController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  alias Exopticon.Video.VideoUnit

  action_fallback(ExopticonWeb.FallbackController)

  def index(conn, %{"camera_id" => camera_id}) do
    video_units = Video.list_video_units_by_camera(camera_id)
    render(conn, "index.json", video_units: video_units)
  end

  def create(conn, %{"video_unit" => video_unit_params}) do
    with {:ok, %VideoUnit{} = video_unit} <- Video.create_video_unit(video_unit_params) do
      conn
      |> put_status(:created)
      |> put_resp_header("location", video_unit_v1_path(conn, :show, video_unit))
      |> render("show.json", video_unit: video_unit)
    end
  end

  def show(conn, %{"id" => id}) do
    video_unit = Video.get_video_unit!(id)
    render(conn, "show.json", video_unit: video_unit)
  end

  def update(conn, %{"id" => id, "video_unit" => video_unit_params}) do
    video_unit = Video.get_video_unit!(id)

    with {:ok, %VideoUnit{} = video_unit} <- Video.update_video_unit(video_unit, video_unit_params) do
      render(conn, "show.json", video_unit: video_unit)
    end
  end

  def delete(conn, %{"id" => id}) do
    video_unit = Video.get_video_unit!(id)

    with {:ok, %VideoUnit{}} <- Video.delete_video_unit(video_unit) do
      send_resp(conn, :no_content, "")
    end
  end

  def between(conn, %{"camera_id" => id, "begin_time" => begin_time, "end_time" => end_time}) do
    videos = Video.list_video_units_between(id, begin_time, end_time)

    render(conn, "index.json", video_units: videos)
  end
end
