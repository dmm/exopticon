defmodule ExopticonWeb.V1.CameraController do
  use ExopticonWeb, :controller

  alias Exopticon.Video
  alias Exopticon.Video.Camera

  plug :scrub_params, "post" when action in [:create, :update]

  def index(conn, _params) do
    cameras = Video.list_cameras()
    render(conn, "index.json", cameras: cameras)
  end

end
