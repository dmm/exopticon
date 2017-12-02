defmodule ExopticonWeb.V1.CameraController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  #  alias Exopticon.Video.Camera

  plug(:scrub_params, "post" when action in [:create, :update])

  def index(conn, _params) do
    cameras = Video.list_cameras()
    render(conn, "index.json", cameras: cameras)
  end

  def relativeMove(conn, %{ "id" => id } = params) do
    IO.puts("getting camera")
    camera = Video.get_camera!(id)
    IO.puts("got camera")
    url = Exvif.Cam.cam_url(camera.ip, camera.onvif_port)
    %{ "x" => x, "y" => y} = params
    IO.puts("beginning request")
    ret = Exvif.Cam.request_ptz_relative_move(url, camera.username, camera.password,
      camera.ptz_profile_token, x, y)
    IO.puts("request done")
    json(conn, %{id: id})
  end
end
