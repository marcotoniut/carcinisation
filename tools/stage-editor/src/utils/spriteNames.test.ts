import { describe, expect, it } from "vitest"
import type { DestructibleType } from "@/types/generated/DestructibleType"
import type { EnemyType } from "@/types/generated/EnemyType"
import type { ObjectType } from "@/types/generated/ObjectType"
import type { PickupType } from "@/types/generated/PickupType"
import {
  DESTRUCTIBLE_SPRITE_NAMES,
  ENEMY_SPRITE_NAMES,
  OBJECT_SPRITE_NAMES,
  PICKUP_SPRITE_NAMES,
} from "./spriteNames"

describe("spriteNames single source of truth", () => {
  it("covers all enemy types with non-empty base names", () => {
    const enemyTypes: EnemyType[] = [
      "Mosquito",
      "Spidey",
      "Tardigrade",
      "Marauder",
      "Spidomonsta",
      "Kyle",
    ]
    expect(Object.keys(ENEMY_SPRITE_NAMES).sort()).toEqual(enemyTypes.sort())
    enemyTypes.forEach((enemy) => {
      expect(ENEMY_SPRITE_NAMES[enemy]).toBeTruthy()
    })
  })

  it("covers all object types with non-empty base names", () => {
    const objectTypes: ObjectType[] = [
      "BenchBig",
      "BenchSmall",
      "Fibertree",
      "RugparkSign",
    ]
    expect(Object.keys(OBJECT_SPRITE_NAMES).sort()).toEqual(objectTypes.sort())
    objectTypes.forEach((objectType) => {
      expect(OBJECT_SPRITE_NAMES[objectType]).toBeTruthy()
    })
  })

  it("covers all pickup types with non-empty base names", () => {
    const pickupTypes: PickupType[] = ["SmallHealthpack", "BigHealthpack"]
    expect(Object.keys(PICKUP_SPRITE_NAMES).sort()).toEqual(pickupTypes.sort())
    pickupTypes.forEach((pickupType) => {
      expect(PICKUP_SPRITE_NAMES[pickupType]).toBeTruthy()
    })
  })

  it("covers all destructible types with non-empty base names", () => {
    const destructibleTypes: DestructibleType[] = [
      "Lamp",
      "Trashcan",
      "Crystal",
      "Mushroom",
    ]
    expect(Object.keys(DESTRUCTIBLE_SPRITE_NAMES).sort()).toEqual(
      destructibleTypes.sort(),
    )
    destructibleTypes.forEach((destructibleType) => {
      expect(DESTRUCTIBLE_SPRITE_NAMES[destructibleType]).toBeTruthy()
    })
  })
})
