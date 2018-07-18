defmodule ExopticonWeb.V1.CameraController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  #  alias Exopticon.Video.Camera

  plug(:scrub_params, "post" when action in [:create, :update])

  def index(conn, _params) do
    cameras = Video.list_cameras()

    cameras2 =
      Enum.map(cameras, fn c ->
        Map.put(c, :link, ExopticonWeb.Router.Helpers.camera_path(conn, :show, c.id))
      end)

    render(conn, "index.json", cameras: Enum.sort(cameras2))
  end

  def show(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    camera2 = Map.put(camera, :link, ExopticonWeb.Router.Helpers.camera_path(conn, :show, id))
    render(conn, "show.json", camera: camera2)
  end

  def relativeMove(conn, %{"id" => id} = params) do
    %{"x" => x, "y" => y} = params
    camera = Video.get_camera!(id)

    Video.relative_move_camera(camera, x, y)

    json(conn, %{id: id})
  end

  def video(conn, %{"id" => id, "begin_time" => begin_time, "end_time" => end_time}) do
    videos = Video.list_video_units_between(id, begin_time, end_time)

    render(conn, "index.json", video_units: videos)
  end
end
