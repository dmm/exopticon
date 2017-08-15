defmodule ExopticonWeb.V1.FileController do
  use ExopticonWeb, :controller

  alias Exopticon.Video
  alias Exopticon.Video.Camera

  plug :scrub_params, "post" when action in [:create, :update]

  def index(conn, %{"camera_id" => camera_id}) do
    files = Video.get_files_between(
    render(conn, "index.json", files: files)
  end
end
