defmodule ExopticonWeb.CameraController do
  use ExopticonWeb, :controller

  alias Exopticon.Video
  alias Exopticon.Video.Camera

  def index(conn, _params) do
    cameras = Video.list_cameras()
    render(conn, "index.html", cameras: cameras)
  end

  def new(conn, _params) do
    changeset = Video.change_camera(%Camera{})
    render(conn, "new.html", changeset: changeset)
  end

  def create(conn, %{"camera" => camera_params}) do
    case Video.create_camera(camera_params) do
      {:ok, camera} ->
        conn
        |> put_flash(:info, "Camera created successfully.")
        |> redirect(to: camera_path(conn, :show, camera))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "new.html", changeset: changeset)
    end
  end

  def show(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    render(conn, "show.html", camera: camera)
  end

  def edit(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    changeset = Video.change_camera(camera)
    render(conn, "edit.html", camera: camera, changeset: changeset)
  end

  def update(conn, %{"id" => id, "camera" => camera_params}) do
    camera = Video.get_camera!(id)

    case Video.update_camera(camera, camera_params) do
      {:ok, camera} ->
        conn
        |> put_flash(:info, "Camera updated successfully.")
        |> redirect(to: camera_path(conn, :show, camera))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "edit.html", camera: camera, changeset: changeset)
    end
  end

  def delete(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    {:ok, _camera} = Video.delete_camera(camera)

    conn
    |> put_flash(:info, "Camera deleted successfully.")
    |> redirect(to: camera_path(conn, :index))
  end
end
