// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "JsonObjectConverter.h"

namespace Banette::Transport::Json
{
	template <typename T>
	concept CIsUStruct =
		requires
		{
			{ std::remove_cvref_t<T>::StaticStruct() } -> std::same_as<UScriptStruct*>;
		};

	template <typename T>
	concept CIsUObject =
		std::is_base_of_v<UObject, T>;

	template <typename T>
	bool JsonToCpp(const TSharedPtr<FJsonValue>& JsonValue, T& OutValue)
	{
		static_assert(sizeof(T) == 0,
		              "JsonToCpp: Type not implemented — please add a specialization or overload.");
		return false;
	};

	template <>
	inline bool JsonToCpp(const TSharedPtr<FJsonValue>& Value, FString& Out)
	{
		if (!Value.IsValid())
			return false;

		switch (Value->Type)
		{
		case EJson::String:
			Out = Value->AsString();
			return true;

		case EJson::Number:
			Out = FString::SanitizeFloat(Value->AsNumber());
			return true;

		case EJson::Boolean:
			Out = Value->AsBool() ? TEXT("true") : TEXT("false");
			return true;

		default:
			return false;
		}
	}

	template <typename T>
		requires std::is_arithmetic_v<T>
	bool JsonToCpp(const TSharedPtr<FJsonValue>& Value, T& Out)
	{
		if (!Value.IsValid())
			return false;

		if (Value->Type == EJson::Number)
		{
			Out = Value->AsNumber();
			return true;
		}

		return false;
	}

	template <CIsUStruct T>
	bool JsonToCpp(const TSharedPtr<FJsonValue>& Value, T& Out)
	{
		if (!Value.IsValid() || Value->Type != EJson::Object)
			return false;

		const TSharedPtr<FJsonObject> JsonObject = Value->AsObject();
		if (!JsonObject.IsValid())
			return false;

		return FJsonObjectConverter::JsonObjectToUStruct(
			JsonObject.ToSharedRef(),
			T::StaticStruct(),
			&Out,
			0, 0
		);
	}

	template <CIsUObject T>
	bool JsonToCpp(const TSharedPtr<FJsonValue>& Value, T*& Out)
	{
		if (!Value.IsValid() || Value->Type != EJson::Object)
			return false;

		const TSharedPtr<FJsonObject> JsonObject = Value->AsObject();
		if (!JsonObject.IsValid())
			return false;

		if (!Out)
			Out = NewObject<T>();

		const UClass* Clazz = Out->GetClass();

		return FJsonObjectConverter::JsonObjectToUStruct(
			JsonObject.ToSharedRef(),
			Clazz,
			Out,
			0, 0
		);
	}


	template <typename T>
	bool JsonToCpp(const TSharedPtr<FJsonValue>& Value, TArray<T>& Out)
	{
		if (!Value.IsValid() || Value->Type != EJson::Array)
			return false;

		const auto& JsonArr = Value->AsArray();
		Out.Reset();
		Out.Reserve(JsonArr.Num());

		for (const TSharedPtr<FJsonValue>& Item : JsonArr)
		{
			if constexpr (std::is_pointer_v<T>)
			{
				using Pointee = std::remove_pointer_t<T>;

				if constexpr (std::is_base_of_v<UObject, Pointee>)
				{
					Pointee* Elem = nullptr;
					if (!JsonToCpp(Item, Elem))
						return false;
					Out.Add(Elem);
				}
				else
				{
					T Elem = nullptr;
					if (!JsonToCpp(Item, Elem))
						return false;
					Out.Add(Elem);
				}
			}
			else
			{
				T Elem{};
				if (!JsonToCpp(Item, Elem))
					return false;
				Out.Add(MoveTemp(Elem));
			}
		}

		return true;
	}
}
