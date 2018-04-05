defmodule ExopticonWeb.V1.CameraController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  #  alias Exopticon.Video.Camera

  plug(:scrub_params, "post" when action in [:create, :update])

  def index(conn, _params) do
    cameras = Video.list_cameras()
    cameras2 = Enum.map(cameras, fn(c) -> Map.put(c, :link, ExopticonWeb.Router.Helpers.camera_path(conn, :show, c.id)) end)
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

  def availability(conn, %{"id" => id} = params) do
    end_time_string = Map.get(params, "end_time", Timex.now() |> Timex.format("{ISO:Extended}"))

    {:ok, end_time} = Timex.parse(end_time_string, "{ISO:Extended}")

    begin_time_string =
      Map.get(
        params,
        "begin_time",
        end_time |> Timex.shift(hours: -6) |> Timex.format("{ISO:Extended}")
      )

    {:ok, begin_time} = Timex.parse(begin_time_string, "{ISO:Extended}")

    chunks = Video.get_video_coverage(id, begin_time, end_time)
    files = Video.get_files_between(id, begin_time, end_time)

    files2 = Enum.map(files, fn f -> Map.from_struct(f) |> Map.delete(:__meta__) end)

    json(conn, %{
      camera_id: id,
      begin_time: chunks.begin_time,
      end_time: chunks.end_time,
      availability: chunks.availability,
      files: files2
    })
  end
end
