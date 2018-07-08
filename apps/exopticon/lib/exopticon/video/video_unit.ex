defmodule Exopticon.Video.VideoUnit do
  @moduledoc """
  Provides schema for Video.VideoUnit
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.VideoUnit


  schema "video_units" do
    field :begin_monotonic, :integer
    field :begin_time, :utc_datetime
    field :end_monotonic, :integer
    field :end_time, :utc_datetime
    field :monotonic_index, :integer
    field :camera_id, :id
    has_many :files, Exopticon.Video.File

    timestamps()
  end

  @doc false
  def changeset(%VideoUnit{} = video_unit, attrs) do
    video_unit
    |> cast(attrs, [:begin_time, :end_time, :begin_monotonic, :end_monotonic, :monotonic_index])
    |> validate_required([:begin_time, :end_time, :begin_monotonic, :end_monotonic, :monotonic_index])
  end
end
