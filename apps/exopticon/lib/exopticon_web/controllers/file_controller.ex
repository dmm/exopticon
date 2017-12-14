defmodule ExopticonWeb.FileController do
  use ExopticonWeb, :controller

  import Ecto.Query

  plug(:authenticate_user)

  alias Exopticon.Video
  alias Exopticon.Video.File

  def index(conn, _params) do
    files = Video.list_files()
    render(conn, "index.html", files: files)
  end

  def new(conn, _params) do
    changeset = Video.change_file(%File{})
    render(conn, "new.html", changeset: changeset)
  end

  def create(conn, %{"file" => file_params}) do
    case Video.create_file(file_params) do
      {:ok, file} ->
        conn
        |> put_flash(:info, "File created successfully.")
        |> redirect(to: file_path(conn, :show, file))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "new.html", changeset: changeset)
    end
  end

  def show(conn, %{"id" => id}) do
    file = Video.get_file!(id)
    render(conn, "show.html", file: file)
  end

  def edit(conn, %{"id" => id}) do
    file = Video.get_file!(id)
    changeset = Video.change_file(file)
    render(conn, "edit.html", file: file, changeset: changeset)
  end

  def update(conn, %{"id" => id, "file" => file_params}) do
    file = Video.get_file!(id)

    case Video.update_file(file, file_params) do
      {:ok, file} ->
        conn
        |> put_flash(:info, "File updated successfully.")
        |> redirect(to: file_path(conn, :show, file))

      {:error, %Ecto.Changeset{} = changeset} ->
        render(conn, "edit.html", file: file, changeset: changeset)
    end
  end

  def delete(conn, %{"id" => id}) do
    file = Video.get_file!(id)
    {:ok, _file} = Video.delete_file(file)

    conn
    |> put_flash(:info, "File deleted successfully.")
    |> redirect(to: file_path(conn, :index))
  end

  def indentify_for_deletion(camera_group_id) do
    max_size = Video.get_camera_group!(camera_group_id).max_storage_size * 1024 * 1024 * 1024
    current_size = Exopticon.Repo.one(from(f in Exopticon.Video.File, select: sum(f.size)))
    size_to_remove = current_size - max_size

    files = Video.list_files()
  end
end
