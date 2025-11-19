// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Experimental/UnifiedError/UnifiedError.h"


namespace Banette::Core
{
	namespace Error
	{
		template <typename T>
		concept CUnifiedError = std::is_base_of_v<UE::UnifiedError::FError, T>;
	}

	// template <typename ValueT, Error::CUnifiedError ErrorT = UE::UnifiedError::FError>
	// using TResult = TValueOrError<ValueT, ErrorT>;

	template <typename V, Error::CUnifiedError E = UE::UnifiedError::FError>
	class TResult : public TValueOrError<V, E>
	{
	public:
		~TResult() = default;

		TResult() : TValueOrError<V, E>(MakeError(E(0, 0, nullptr)))
		{
		}

		using TValueOrError<V, E>::TValueOrError;
	};
}
