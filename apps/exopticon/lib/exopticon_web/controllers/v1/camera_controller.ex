defmodule ExopticonWeb.V1.CameraController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  #  alias Exopticon.Video.Camera

  plug(:scrub_params, "post" when action in [:create, :update])

  def index(conn, _params) do
    cameras = Video.list_cameras()
    render(conn, "index.json", cameras: Enum.sort(cameras))
  end

  def show(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    render(conn, "show.json", camera: camera)
  end

  def relativeMove(conn, %{"id" => id} = params) do
    %{"x" => x, "y" => y} = params
    camera = Video.get_camera!(id)

    Video.relative_move_camera(camera, x, y)

    json(conn, %{id: id})
  end
end
