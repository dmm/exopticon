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

defmodule ExopticonWeb.CameraController do
  use ExopticonWeb, :controller
  use Timex

  plug(:authenticate_user)

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

  def playback(conn, %{"id" => id}) do
    camera = Video.get_camera!(id)
    render(conn, "playback.html", camera: camera)
  end

  defp date_to_timespan(date) do
    day_start = date |> Timex.set(hour: 00, minute: 00, second: 00)
    day_end = day_start |> Timex.set(hour: 23, minute: 59, second: 59)

    {day_start, day_end}
  end

  defp get_snapshot_counts(id, date) do
    {day_start, day_end} = date_to_timespan(date)
    start_utc = day_start |> Timezone.convert("Z")
    end_utc = day_end |> Timezone.convert("Z")

    today_count = Video.get_snapshot_count(id, start_utc, end_utc)

    yesterday_count =
      Video.get_snapshot_count(
        id,
        start_utc |> Timex.shift(days: -1),
        end_utc |> Timex.shift(days: -1)
      )

    {today_count, yesterday_count}
  end

  def snapshots(conn, %{"id" => id} = parameters) do
    {offset, _} = Integer.parse(parameters["offset"] || "0")
    date = Timex.now(conn.assigns.current_user.timezone)
    {day_start, day_end} = date_to_timespan(date)
    {today_count, yesterday_count} = get_snapshot_counts(id, date)

    camera = Video.get_camera!(id)
    snapshots = Video.list_recent_snapshots(id, 24, offset)
    user = conn.assigns.current_user

    snapshot_count =
      Video.get_snapshot_count(
        id,
        Timex.now() |> Timex.shift(years: -1000),
        Timex.now() |> Timex.shift(years: 1000)
      )

    render(
      conn,
      "snapshots.html",
      title: "Latest Snapshots",
      camera: camera,
      snapshots: snapshots,
      user: user,
      today_count: today_count,
      yesterday_count: yesterday_count,
      prev_count: offset,
      next_count: snapshot_count - (offset + 24)
    )
  end

  def snapshots_today(conn, %{"id" => id}) do
    date = Timex.now(conn.assigns.current_user.timezone)
    {day_start, day_end} = date_to_timespan(date)
    {today_count, yesterday_count} = get_snapshot_counts(id, date)

    camera = Video.get_camera!(id)
    snapshots = Video.list_snapshots_between(id, day_start, day_end)
    user = conn.assigns.current_user

    render(
      conn,
      "snapshots.html",
      title: "Today's Snapshots",
      camera: camera,
      snapshots: snapshots,
      user: user,
      today_count: today_count,
      yesterday_count: yesterday_count,
      prev_count: 0,
      next_count: 0
    )
  end

  def snapshots_yesterday(conn, %{"id" => id}) do
    date = Timex.now(conn.assigns.current_user.timezone)
    {day_start, day_end} = date_to_timespan(date)
    {today_count, yesterday_count} = get_snapshot_counts(id, date)

    camera = Video.get_camera!(id)

    snapshots =
      Video.list_snapshots_between(
        id,
        day_start |> Timex.shift(days: -1),
        day_end |> Timex.shift(days: -1)
      )

    user = conn.assigns.current_user

    render(
      conn,
      "snapshots.html",
      title: "Yesterday's Snapshots",
      camera: camera,
      snapshots: snapshots,
      user: user,
      today_count: today_count,
      yesterday_count: yesterday_count,
      prev_count: 0,
      next_count: 0
    )
  end
end
