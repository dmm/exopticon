defmodule Exopticon.Repo.Migrations.CreateFiles do
  use Ecto.Migration

  def change do
    create table(:files) do
      add(:filename, :string)
      add(:size, :integer)
      add(:time, :tsrange)
      add(:begin_monotonic, :bigint)
      add(:end_monotonic, :bigint)
      add(:monotonic_index, :integer)
      add(:camera_id, references(:cameras, on_delete: :nothing))

      timestamps()
    end

    create(index(:files, [:camera_id]))
  end
end
