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

defmodule Exopticon.Video do
  @moduledoc """
  The Video context.
  """

  import File
  import Ecto.Query, warn: false
  require Logger

  alias Exopticon.Repo
  alias Exopticon.Video.CameraGroup
  alias Exopticon.Video.VideoUnit
  alias Exvif.Cam

  @doc """
  Returns the list of camera_groups.

  ## Examples

      iex> list_camera_groups()
      [%CameraGroup{}, ...]

  """
  def list_camera_groups do
    Repo.all(CameraGroup)
  end

  @doc """
  Gets a single camera_group.

  Raises `Ecto.NoResultsError` if the Camera group does not exist.

  ## Examples

      iex> get_camera_group!(123)
      %CameraGroup{}

      iex> get_camera_group!(456)
      ** (Ecto.NoResultsError)

  """
  def get_camera_group!(id), do: Repo.get!(CameraGroup, id)

  @doc """
  Creates a camera_group.

  ## Examples

      iex> create_camera_group(%{field: value})
      {:ok, %CameraGroup{}}

      iex> create_camera_group(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_camera_group(attrs \\ %{}) do
    %CameraGroup{}
    |> CameraGroup.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a camera_group.

  ## Examples

      iex> update_camera_group(camera_group, %{field: new_value})
      {:ok, %CameraGroup{}}

      iex> update_camera_group(camera_group, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_camera_group(%CameraGroup{} = camera_group, attrs) do
    camera_group
    |> CameraGroup.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a CameraGroup.

  ## Examples

      iex> delete_camera_group(camera_group)
      {:ok, %CameraGroup{}}

      iex> delete_camera_group(camera_group)
      {:error, %Ecto.Changeset{}}

  """
  def delete_camera_group(%CameraGroup{} = camera_group) do
    Repo.delete(camera_group)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking camera_group changes.

  ## Examples

      iex> change_camera_group(camera_group)
      %Ecto.Changeset{source: %CameraGroup{}}

  """
  def change_camera_group(%CameraGroup{} = camera_group) do
    CameraGroup.changeset(camera_group, %{})
  end

  alias Exopticon.Video.Camera

  @doc """
  Returns the list of cameras.

  ## Examples

      iex> list_cameras()
      [%Camera{}, ...]

  """
  def list_cameras do
    Repo.all(Camera)
  end

  @doc """
  Gets a single camera.

  Raises `Ecto.NoResultsError` if the Camera does not exist.

  ## Examples

      iex> get_camera!(123)
      %Camera{}

      iex> get_camera!(456)
      ** (Ecto.NoResultsError)

  """
  def get_camera!(id), do: Repo.get!(Camera, id)

  @doc """
  Creates a camera.

  ## Examples

      iex> create_camera(%{field: value})
      {:ok, %Camera{}}

      iex> create_camera(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_camera(attrs \\ %{}) do
    ret =
      %Camera{}
      |> Camera.changeset(attrs)
      |> Repo.insert()

    ret
  end

  @doc """
  Updates a camera.

  ## Examples

      iex> update_camera(camera, %{field: new_value})
      {:ok, %Camera{}}

      iex> update_camera(camera, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_camera(%Camera{} = camera, attrs) do
    ret =
      camera
      |> Camera.changeset(attrs)
      |> Repo.update()

    new_mode = Map.get(attrs, "mode")
    {result, _} = ret

    if result == :ok and new_mode == "enabled" do
      IO.puts("STARTING CAMERA")
      start_camera(camera.id)
    end

    if result == :ok and new_mode == "disabled" do
      IO.puts("STOPPING CAMERA")
      stop_camera(camera.id)
    end

    ret
  end

  @doc """
  Deletes a Camera.

  ## Examples

      iex> delete_camera(camera)
      {:ok, %Camera{}}

      iex> delete_camera(camera)
      {:error, %Ecto.Changeset{}}

  """
  def delete_camera(%Camera{} = camera) do
    Repo.delete(camera)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking camera changes.

  ## Examples

      iex> change_camera(camera)
      %Ecto.Changeset{source: %Camera{}}

  """
  def change_camera(%Camera{} = camera) do
    Camera.changeset(camera, %{})
  end

  @doc """
  Requests a relative move from the camera.
  """
  def relative_move_camera(%Camera{ptz_type: "onvif"} = camera, x, y) do
    url = Cam.cam_url(camera.ip, camera.onvif_port)

    Cam.request_ptz_relative_move(
      url,
      camera.username,
      camera.password,
      camera.ptz_profile_token,
      x,
      y
    )
  end

  def relative_move_camera(%Camera{ptz_type: "onvif_continuous"} = camera, x, y) do
    url = Cam.cam_url(camera.ip, camera.onvif_port)

    Cam.request_ptz_continuous_move(
      url,
      camera.username,
      camera.password,
      camera.ptz_profile_token,
      x,
      y
    )

    Process.sleep(1000)
    Cam.request_ptz_stop(url, camera.username, camera.password, camera.ptz_profile_token)
  end

  alias Exopticon.Video.File

  @doc """
  Returns the list of files.

  ## Examples

      iex> list_files()
      [%File{}, ...]

  """
  def list_files do
    Repo.all(File)
  end

  @doc """
  Returns list of files for single camera.

  ## Examples
  iex> list_files(9)
  [%File{}, ...]
  """
  def list_files(camera_id) do
    query = from(f in File, where: f.camera_id == ^camera_id)

    Repo.all(query)
  end

  @doc """
  Gets a single file.

  Raises `Ecto.NoResultsError` if the File does not exist.

  ## Examples

      iex> get_file!(123)
      %File{}

      iex> get_file!(456)
      ** (Ecto.NoResultsError)

  """
  def get_file!(id), do: Repo.get!(File, id)

  @doc """
  Creates a file.

  ## Examples

      iex> create_file(%{field: value})
      {:ok, %File{}}

      iex> create_file(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_file(attrs \\ %{}) do
    %File{}
    |> File.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a file.

  ## Examples

      iex> update_file(file, %{field: new_value})
      {:ok, %File{}}

      iex> update_file(file, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_file(%File{} = file, attrs) do
    file
    |> File.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a File.

  ## Examples

      iex> delete_file(file)
      {:ok, %File{}}

      iex> delete_file(file)
      {:error, %Ecto.Changeset{}}

  """
  def delete_file(%File{} = file) do
    Repo.delete(file)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking file changes.

  ## Examples

      iex> change_file(file)
      %Ecto.Changeset{source: %File{}}

  """
  def change_file(%File{} = file) do
    File.changeset(file, %{})
  end

  @doc """
  Returns video units for single camera
  """
  def get_video_between(camera_id, begin_time, end_time) do
    query =
      from(
        v in VideoUnit,
        where:
          v.camera_id == ^camera_id and ^end_time >= v.begin_time and ^begin_time <= v.end_time and
            not is_nil(v.end_monotonic),
        order_by: [asc: v.monotonic_index, asc: v.begin_monotonic],
        preload: [:files]
      )

    Repo.all(query) || []
  end

  def get_total_video_size(camera_group_id) do
    query =
      from(
        f in File,
        join: vu in VideoUnit,
        on: f.video_unit_id == vu.id,
        join: c in Camera,
        on: c.id == vu.camera_id,
        select: sum(f.size),
        where: c.camera_group_id == ^camera_group_id
      )

    Repo.one(query) || 0
  end

  def get_file_for_time(camera_id, time) do
    query =
      from(
        f in File,
        where: f.camera_id == ^camera_id and f.begin_time <= ^time and f.end_time >= ^time
      )

    Repo.one(query)
  end

  def get_oldest_files_in_group(camera_group_id, count \\ 100) do
    query =
      from(
        f in File,
        join: vu in VideoUnit,
        on: f.video_unit_id == vu.id,
        join: c in Camera,
        on: vu.camera_id == c.id,
        where: c.camera_group_id == ^camera_group_id,
        order_by: [asc: vu.monotonic_index, asc: vu.begin_monotonic],
        limit: ^count
      )

    Repo.all(query)
  end

  def start_camera(camera_id) do
    query =
      from(
        c in Camera,
        where: c.id == ^camera_id,
        preload: [:camera_group]
      )

    camera = Repo.one(query)
    Exopticon.CameraSupervisor.start_all_cameras([camera])
  end

  def stop_camera(camera_id) do
    Exopticon.CameraSupervisor.stop_camera(camera_id)
  end

  alias Exopticon.Video.VideoUnit

  @doc """
  Returns the list of video_units.

  ## Examples

      iex> list_video_units()
      [%VideoUnit{}, ...]

  """
  def list_video_units do
    Repo.all(VideoUnit)
  end

  @doc """
  Returns video_units belonging to given camera
  """
  def list_video_units_by_camera(camera_id) do
    query =
      from(
        v in VideoUnit,
        where: v.camera_id == ^camera_id and not is_nil(v.end_monotonic),
        preload: [:files],
        order_by: [asc: v.monotonic_index, asc: v.begin_monotonic]
      )

    Repo.all(query) || []
  end

  @doc """
  Returns video units for a given camera between two times
  """
  def list_video_units_between(camera_id, begin_time, end_time) do
    query =
      from(
        v in VideoUnit,
        where:
          v.camera_id == ^camera_id and not is_nil(v.end_monotonic) and ^begin_time <= v.end_time and
            ^end_time >= v.begin_time,
        preload: [:files],
        order_by: [asc: v.monotonic_index, asc: v.begin_monotonic]
      )

    Repo.all(query) || []
  end

  @doc """
  Gets a single video_unit.

  Raises `Ecto.NoResultsError` if the Video unit does not exist.

  ## Examples

      iex> get_video_unit!(123)
      %VideoUnit{}

      iex> get_video_unit!(456)
      ** (Ecto.NoResultsError)

  """
  def get_video_unit!(id), do: Repo.get!(VideoUnit, id)

  @doc """
  Creates a video_unit.

  ## Examples

      iex> create_video_unit(%{field: value})
      {:ok, %VideoUnit{}}

      iex> create_video_unit(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_video_unit(attrs \\ %{}) do
    %VideoUnit{}
    |> VideoUnit.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a video_unit.

  ## Examples

      iex> update_video_unit(video_unit, %{field: new_value})
      {:ok, %VideoUnit{}}

      iex> update_video_unit(video_unit, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_video_unit(%VideoUnit{} = video_unit, attrs) do
    video_unit
    |> VideoUnit.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a VideoUnit.

  ## Examples

      iex> delete_video_unit(video_unit)
      {:ok, %VideoUnit{}}

      iex> delete_video_unit(video_unit)
      {:error, %Ecto.Changeset{}}

  """
  def delete_video_unit(%VideoUnit{} = video_unit) do
    Repo.delete(video_unit)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking video_unit changes.

  ## Examples

      iex> change_video_unit(video_unit)
      %Ecto.Changeset{source: %VideoUnit{}}

  """
  def change_video_unit(%VideoUnit{} = video_unit) do
    VideoUnit.changeset(video_unit, %{})
  end

  alias Exopticon.Video.Annotation

  @doc """
  Returns the list of annotations.

  ## Examples

      iex> list_annotations()
      [%Annotation{}, ...]

  """
  def list_annotations do
    Repo.all(Annotation)
  end

  def list_unframed_snapshots do
    query =
      from(
        a in Exopticon.Video.Annotation,
        join: vu in assoc(a, :video_unit),
        join: f in assoc(vu, :files),
        join: c in assoc(vu, :camera),
        join: cg in assoc(c, :camera_group),
        where: is_nil(a.hd_filename) and a.key == "snapshot",
        preload: [video_unit: {vu, camera: {c, camera_group: cg}, files: f}]
      )

    Repo.all(query)
  end

  @doc """
  Gets a single annotation.

  Raises `Ecto.NoResultsError` if the Annotation does not exist.

  ## Examples

      iex> get_annotation!(123)
      %Annotation{}

      iex> get_annotation!(456)
      ** (Ecto.NoResultsError)

  """
  def get_annotation!(id), do: Repo.get!(Annotation, id)

  @doc """
  Creates a annotation.

  ## Examples

      iex> create_annotation(%{field: value})
      {:ok, %Annotation{}}

      iex> create_annotation(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_annotation(attrs \\ %{}) do
    ret =
      %Annotation{}
      |> Annotation.changeset(attrs)
      |> Repo.insert()

    ret
  end

  @doc """
  Updates a annotation.

  ## Examples

      iex> update_annotation(annotation, %{field: new_value})
      {:ok, %Annotation{}}

      iex> update_annotation(annotation, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_annotation(%Annotation{} = annotation, attrs) do
    annotation
    |> Annotation.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a Annotation.

  ## Examples

      iex> delete_annotation(annotation)
      {:ok, %Annotation{}}

      iex> delete_annotation(annotation)
      {:error, %Ecto.Changeset{}}

  """
  def delete_annotation(%Annotation{} = annotation) do
    sd = annotation.sd_filename
    hd = annotation.sd_filename
    {:ok, ret} = Repo.delete(annotation)
    rm(sd)
    rm(hd)
    {:ok, ret}
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking annotation changes.

  ## Examples

      iex> change_annotation(annotation)
      %Ecto.Changeset{source: %Annotation{}}

  """
  def change_annotation(%Annotation{} = annotation) do
    Annotation.changeset(annotation, %{})
  end

  @doc """
  Returns a given number of snapshot annotations from the specified camera.
  """
  def list_recent_snapshots(camera_id, snapshot_count, offset \\ 0) do
    query =
      from(
        a in Exopticon.Video.Annotation,
        join: vu in assoc(a, :video_unit),
        where: vu.camera_id == ^camera_id and a.key == "snapshot",
        order_by: [desc: vu.monotonic_index, desc: vu.begin_monotonic],
        limit: ^snapshot_count,
        offset: ^offset,
        preload: [video_unit: vu]
      )

    Repo.all(query) || []
  end

  @doc """
  Returns snapshot between given dates
  """
  def list_snapshots_between(camera_id, begin_date, end_date) do
    # THIS IS REALLY BROKEN. WE WILL FIX ONCE WE STORE ACTUAL annotation TIMESTAMPS
    query =
      from(
        a in Exopticon.Video.Annotation,
        join: vu in assoc(a, :video_unit),
        where:
          vu.camera_id == ^camera_id and a.key == "snapshot" and vu.begin_time >= ^begin_date and
            vu.begin_time <= ^end_date,
        order_by: [desc: vu.monotonic_index, desc: vu.begin_monotonic],
        preload: [video_unit: vu]
      )

    Repo.all(query) || []
  end

  @doc """
  Returns snapshot counts for date and camera_id.
  """
  def get_snapshot_count(camera_id, begin_time, end_time) do
    Logger.info("Counting snapshots for #{camera_id} between #{begin_time} and #{end_time}")
    # THIS IS REALLY BROKEN. WE WILL FIX ONCE WE STORE ACTUAL annotation TIMESTAMPS
    query =
      from(
        a in Exopticon.Video.Annotation,
        join: vu in assoc(a, :video_unit),
        where:
          vu.camera_id == ^camera_id and a.key == "snapshot" and vu.begin_time <= ^end_time and
            vu.begin_time >= ^begin_time,
        order_by: [desc: vu.monotonic_index, desc: vu.begin_monotonic],
        preload: [video_unit: vu]
      )

    Repo.aggregate(query, :count, :id)
  end
end
