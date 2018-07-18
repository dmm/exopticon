 # This file is a part of Exopticon, a free video surveillance tool. Visit
 # https://exopticon.org for more information.
 #
 # Copyright (C) 2018 David Matthew Mattli
 #
 # This program is free software: you can redistribute it and/or modify
 # it under the terms of the GNU Affero General Public License as published by
 # the Free Software Foundation, either version 3 of the License, or
 # (at your option) any later version.
 #
 # This program is distributed in the hope that it will be useful,
 # but WITHOUT ANY WARRANTY; without even the implied warranty of
 # MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 # GNU Affero General Public License for more details.
 # You should have received a copy of the GNU Affero General Public License
 # along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule ExopticonWeb.FileController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  alias Exopticon.Video.File

  def index(conn, %{}) do
    files = Video.list_files()
    render(conn, "index.html", files: files)
  end

  def index(conn, %{"camera_id" => camera_id}) do
    files = Video.list_files(camera_id)
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

  def browse(conn, %{"camera_id" => camera_id}) do
    render(conn, "browse.html", camera_id: camera_id)
  end
end
