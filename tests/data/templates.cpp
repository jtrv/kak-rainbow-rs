#include <vector>
#include <map>

template<typename T>
struct Wrapper {
    std::vector<std::map<int, T>> items;

    T *get(int i) { return &items[0][i]; }
};

bool compare(int a, int b)
{
    bool lt = a < b; // the ';' kills the template candidate '<'
    return lt && b > 0;
}

int main()
{
    Wrapper<std::vector<int>> w;
    auto p = w.get(0);
    if (p != nullptr && *p > compare(1, 2)) {
        return (1 + 2) * 3;
    }
    auto fn = [](int x) -> int { return x; };
    return fn(0);
}
