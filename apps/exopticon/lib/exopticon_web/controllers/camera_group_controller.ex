defmodule ExopticonWeb.CameraGroupController do
  use ExopticonWeb, :controller

  alias Exopticon.Video
  alias Exopticon.Video.CameraGroup

  def index(conn, _params) do
    camera_groups = Video.list_camera_groups()
    render(conn, "index.html", camera_groups: camera_groups)
  end

  def new(conn, _params) do
    changeset = Video.change_camera_group(%CameraGroup{})
    render(conn, "new.html", changeset: changeset)
  end

  def create(conn, %{"camera_group" => camera_group_params}) do
    case Video.create_camera_group(camera_group_params) do
      {:ok, camera_group} ->
        conn
        |> put_flash(:info, "Camera group created successfully.")
        |> redirect(to: camera_group_path(conn, :show, camera_group))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "new.html", changeset: changeset)
    end
  end

  def show(conn, %{"id" => id}) do
    camera_group = Video.get_camera_group!(id)
    render(conn, "show.html", camera_group: camera_group)
  end

  def edit(conn, %{"id" => id}) do
    camera_group = Video.get_camera_group!(id)
    changeset = Video.change_camera_group(camera_group)
    render(conn, "edit.html", camera_group: camera_group, changeset: changeset)
  end

  def update(conn, %{"id" => id, "camera_group" => camera_group_params}) do
    camera_group = Video.get_camera_group!(id)

    case Video.update_camera_group(camera_group, camera_group_params) do
      {:ok, camera_group} ->
        conn
        |> put_flash(:info, "Camera group updated successfully.")
        |> redirect(to: camera_group_path(conn, :show, camera_group))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "edit.html", camera_group: camera_group, changeset: changeset)
    end
  end

  def delete(conn, %{"id" => id}) do
    camera_group = Video.get_camera_group!(id)
    {:ok, _camera_group} = Video.delete_camera_group(camera_group)

    conn
    |> put_flash(:info, "Camera group deleted successfully.")
    |> redirect(to: camera_group_path(conn, :index))
  end
end
