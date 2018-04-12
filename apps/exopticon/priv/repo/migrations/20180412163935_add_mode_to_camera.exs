defmodule Exopticon.Repo.Migrations.AddModeToCamera do
  use Ecto.Migration

  def change do
    alter table(:cameras) do
      add(:mode, :string, default: "enabled")
    end
  end
end
