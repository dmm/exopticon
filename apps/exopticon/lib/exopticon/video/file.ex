defmodule Exopticon.Video.File do
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.File

  schema "files" do
    field(:begin_time, :utc_datetime)
    field(:end_time, :utc_datetime)
    field(:begin_monotonic, :integer)
    field(:end_monotonic, :integer)
    field(:filename, :string)
    field(:monotonic_index, :integer)
    field(:size, :integer)
    field(:camera_id, :id)

    timestamps()
  end

  @doc false
  def changeset(%File{} = file, attrs) do
    file
    |> cast(attrs, [
      :filename,
      :size,
      :begin_time,
      :end_time,
      :begin_monotonic,
      :end_monotonic,
      :monotonic_index
    ])
    |> validate_required([
      :filename,
      :size,
      :begin_time,
      :end_time,
      :begin_monotonic,
      :end_monotonic,
      :monotonic_index
    ])
  end
end
